// PIMProbe - touch probing for the Nestworks C500.
// Copyright (c) 2026 Konstantin Tcepliaev <f355@f355.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

import { mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import vm from 'node:vm';

export const languages = ['zh_CN', 'sv'];
const source = new URL('../ui/i18n/', import.meta.url);
const uiSource = new URL('../ui/', import.meta.url);
const literal = String.raw`"(?:\\[\s\S]|[^"\\])*"|'(?:\\[\s\S]|[^'\\])*'`;

function withoutComments(text) {
    return text.replace(new RegExp(`${literal}|//[^\\n]*|/\\*[\\s\\S]*?\\*/`, 'g'),
        token => token.startsWith('/') ? token.replace(/[^\n]/g, ' ') : token);
}

function strings(text) {
    return [...text.matchAll(new RegExp(literal, 'g'))].map(match => vm.runInNewContext(match[0]));
}

function isDisplaySource(text) {
    return text.trim() && !/^(?:[XYZAG][+-]?|G\d+|[\d\s+.,/°-]+)$/.test(text);
}

// Recognize literal translation calls and label models used by this UI.
export function extractSources(text, models = true) {
    text = withoutComments(text);
    const sources = new Set();
    const add = value => { if (value.trim()) sources.add(value); };
    for (const match of text.matchAll(new RegExp(`\\bI18n\\.tr\\(\\s*(${literal})(?=\\s*[,\\)])`, 'g')))
        add(strings(match[1])[0]);
    for (const match of text.matchAll(new RegExp(`\\|\\|\\s*(${literal})`, 'g'))) {
        const value = strings(match[1])[0];
        if (/\w\s+\w/.test(value)) add(value);
    }
    for (const match of text.matchAll(new RegExp(`\\b(?:error(?:Text)?|failure|requestError|actionError)\\s*[:=]\\s*(${literal})`, 'g')))
        add(strings(match[1])[0]);
    if (!models) return sources;
    for (const match of text.matchAll(new RegExp(`\\b(?:label|unit|section)\\s*:\\s*(${literal})`, 'g'))) {
        const value = strings(match[1])[0];
        if (isDisplaySource(value)) add(value);
    }
    for (const match of text.matchAll(/\bvar\s+label\s*=\s*([^;]+);/g))
        for (const value of strings(match[1]))
            if (isDisplaySource(value)) add(value);
    for (const match of text.matchAll(new RegExp(`\\[\\s*(?:${literal})(?:\\s*,\\s*(?:${literal}))*\\s*,?\\s*\\]`, 'g')))
        for (const value of strings(match[0]))
            if (isDisplaySource(value) && (/^[A-Z]/.test(value)
                || /\bmodel\s*:\s*$/.test(text.slice(0, match.index)))) add(value);
    for (const value of strings(text))
        if (/%[1-9]\d*/.test(value) && /[A-Za-z]/.test(value)) add(value);
    for (const match of text.matchAll(/\bvar\s+descriptions\s*=\s*\{([\s\S]*?)\};/g))
        for (const entry of match[1].matchAll(new RegExp(`:\\s*(${literal})`, 'g')))
            add(strings(entry[1])[0]);
    for (const match of text.matchAll(/\bfunction\s+probeState\([^)]*\)\s*\{([\s\S]*?)\n\s*\}/g))
        for (const value of match[1].matchAll(new RegExp(`\\breturn\\s+(${literal})`, 'g')))
            add(strings(value[1])[0]);
    for (const match of text.matchAll(new RegExp(`\\.confirm\\(\\s*(${literal})\\s*\\)`, 'g')))
        add(strings(match[1])[0]);
    for (const match of text.matchAll(new RegExp(`\\bcalibrationSaved\\([^?\\n]+\\?\\s*(${literal})\\s*:\\s*(${literal})\\s*\\)`, 'g')))
        for (const value of strings(match[1] + ' ' + match[2])) add(value);
    return sources;
}

export function collectSources(ui = uiSource) {
    const sources = new Map();
    const add = (english, file) => { if (english && !sources.has(english)) sources.set(english, file); };
    for (const file of readdirSync(ui, {recursive: true}).sort()) {
        if (!/\.(?:qml|js)$/.test(file) || /^(?:tests|animations|i18n)\//.test(file) || file === 'HelpPages.js') continue;
        const text = readFileSync(new URL(file, ui), 'utf8');
        for (const english of extractSources(text, file.endsWith('.qml') && !file.startsWith('client/')))
            add(english, 'ui/' + file);
    }

    function script(file) {
        const context = {};
        vm.runInNewContext(readFileSync(new URL(file, ui), 'utf8').replace(/^\.pragma library\s*/m, ''),
            context, {filename: 'ui/' + file, timeout: 1000});
        return context;
    }
    const pages = script('ProbePages.js');
    const review = script('ReviewPlan.js');
    for (const fields of [pages.outside, pages.inside, pages.center, pages.rotary, pages.setup])
        for (const field of fields) {
            add(field.label, 'ui/ProbePages.js');
            add(field.unit || 'mm', 'ui/ProbePages.js');
        }
    const grid = withoutComments(readFileSync(new URL('CenterGrid.qml', ui), 'utf8'));
    const features = [...grid.matchAll(new RegExp(`\\bfeature\\s*:\\s*(${literal})`, 'g'))]
        .map(match => strings(match[1])[0]);
    if (!features.length) throw new Error('No center features found in ui/CenterGrid.qml');
    function plan(config, rotary = false) {
        const description = review.describe(config, rotary);
        for (const phase of description.phases) add(phase, 'ui/ReviewPlan.js');
        for (const option of description.options) {
            add(option.label, 'ui/ReviewPlan.js');
            add(option.unit, 'ui/ReviewPlan.js');
            add(option.section, 'ui/ReviewPlan.js');
        }
        if (!rotary) add(pages.featureName(config), 'ui/ProbePages.js');
    }
    for (const family of ['inside', 'outside'])
        for (const x of [-1, 0, 1])
            for (const y of [-1, 0, 1])
                plan(pages.routine({}, family, {x, y, z: !x && !y}, 54));
    for (const feature of features) {
        plan(pages.routine({}, 'center', feature, 54));
        if (feature !== 'z') add(feature.replace('-', ' ') + ' center', 'service history labels');
    }
    for (const operation of ['axis', 'horizontal', 'vertical', 'verticalNegative']) plan({operation}, true);

    // These labels arrive from the service rather than literal UI models.
    for (const english of ['success', 'failed', 'interrupted', 'Rotary axis calibration', 'Align vertical surface toward Y-',
        'Inside %1 edge', 'Outside %1 edge', 'Inside %1 corner', 'Outside %1 corner'])
        add(english, 'service history labels');
    for (const english of strings(withoutComments(readFileSync(new URL('client/Http.js', ui), 'utf8'))))
        if (/\w\s+\w/.test(english) && /[.!?]$/.test(english)) add(english, 'ui/client/Http.js');
    add(script('client/Http.js').responseError('%1'), 'ui/client/Http.js');
    return sources;
}

export function validateCatalogs(catalogs, sources = collectSources()) {
    const errors = [];
    const parameters = text => [...new Set(text.match(/%[1-9]\d*/g) || [])].sort();
    for (const language of languages) {
        const catalog = catalogs[language];
        if (!catalog || typeof catalog !== 'object' || Array.isArray(catalog)) {
            errors.push(`${language}: missing or invalid catalog`);
            continue;
        }
        for (const [english, translated] of Object.entries(catalog).sort())
            if (typeof translated !== 'string' || !translated.trim()
                || JSON.stringify(parameters(english)) !== JSON.stringify(parameters(translated)))
                errors.push(`${language}: invalid translation or parameters for ${JSON.stringify(english)}`);
        for (const [english, file] of [...sources].sort())
            if (!Object.hasOwn(catalog, english))
                errors.push(`${language}: missing ${JSON.stringify(english)} (${file})`);
    }
    if (errors.length) throw new Error('Translation validation failed:\n' + errors.join('\n'));
}

export function loadCatalogs() {
    const catalogs = {};
    for (const language of languages) {
        const catalog = JSON.parse(readFileSync(new URL(language + '.json', source), 'utf8'));
        catalogs[language] = catalog;
    }
    return catalogs;
}

export function buildCatalog(ui = new URL('../ui/', import.meta.url)) {
    const catalogs = loadCatalogs();
    validateCatalogs(catalogs);
    const notice = readFileSync(new URL('license-header.txt', import.meta.url), 'utf8').trimEnd()
        .split('\n').map(line => line ? '// ' + line : '//').join('\n');
    const destination = new URL('i18n/Catalog.js', ui);
    mkdirSync(new URL('.', destination), {recursive: true});
    writeFileSync(destination, notice + '\n\n.pragma library\n\nvar languages = '
        + JSON.stringify(catalogs, null, 4) + ';\n');
}

if (process.argv[1] === fileURLToPath(import.meta.url)) buildCatalog();
