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

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';
import { collectSources, extractSources, loadCatalogs, validateCatalogs } from './build-i18n.mjs';

const translations = {
    'Move %1 to %2.': 'Till %2: flytta %1.',
    'Limit %1: %1%2 is outside [%3, %4].': 'Gräns %1: %1%2 ligger utanför [%3, %4].',
    'Failed (%1)': 'Misslyckades (%1)',
    '%1; history could not be saved: %2': '%1; historiken kunde inte sparas: %2',
    'Permission denied': 'Åtkomst nekad',
    'Start\nTouch': 'Börja\nProba',
    'Probe': 'Mätprob'
};
const context = vm.createContext({Catalog: {languages: {sv: translations}}});
vm.runInContext(readFileSync(new URL('../ui/i18n/Translate.js', import.meta.url), 'utf8')
    .replace(/^\.(?:pragma|import).*$/gm, ''), context);

test('translations interpolate reordered parameters and fall back to English', () => {
    assert.equal(context.translate('sv', 'Move %1 to %2.', ['X', '-120.5']), 'Till -120.5: flytta X.');
    assert.equal(context.translate('en', 'Move %1 to %2.', ['X', '-120.5']), 'Move X to -120.5.');
    assert.equal(context.translate('sv', 'Unknown %1', [25]), 'Unknown 25');
});

test('formatted service messages preserve signed coordinates and error codes', () => {
    assert.equal(context.message('sv', 'Limit X: X-120.5 is outside [-200, 0].'),
        'Gräns X: X-120.5 ligger utanför [-200, 0].');
    assert.equal(context.message('sv', 'Failed (error:5)'), 'Misslyckades (error:5)');
    assert.equal(context.message('sv', 'Move Probe to -120.5.\nFailed (error:5)'),
        'Till -120.5: flytta Mätprob.\nMisslyckades (error:5)');
    assert.equal(context.message('sv', 'Firmware detail: 123'), 'Firmware detail: 123');
});

test('history failure wrappers translate nested causes and I/O messages without changing error codes', () => {
    const message = 'Failed (error:5); history could not be saved: Permission denied';
    assert.equal(context.message('sv', message),
        'Misslyckades (error:5); historiken kunde inte sparas: Åtkomst nekad');
    assert.equal(context.message('en', message), message);
});

test('multiline labels use their complete translation', () => {
    assert.equal(context.message('sv', 'Start\nTouch'), 'Börja\nProba');
});

test('source extraction reads both quote styles and decodes display escapes', () => {
    const sources = extractSources(String.raw`
        text: I18n.tr('Probe')
        text: Controls.I18n.tr("Operator's settings")
        text: I18n.tr('Probe\'s angle: %1\u00b0', [angle])
        text: I18n.tr(
            "Start\nTouch",
            []
        )
        // I18n.tr('Comment')
        /* I18n.tr("Another comment") */
    `);
    assert.deepEqual([...sources].sort(), ["Operator's settings", 'Probe', "Probe's angle: %1°", 'Start\nTouch'].sort());
});

test('source extraction includes labels, units, string models and conditional templates', () => {
    const sources = extractSources(`
        model: ["Outside", "Inside"]
        model:
            ['yes', 'no']
        model: [{label:"Mean G53", key:"mean"}]
        ["initialTouches", "touches"].forEach(function(key) {})
        var label = round ? ["Span X", "Span Y"][index] : "Width X";
        unit: "mm/min"
        section: "Feeds and backoff"
        text: I18n.tr(inside ? 'Probing inside %1 edge.' : 'Probing outside %1 edge.', [axis])
    `);
    assert.deepEqual([...sources].sort(), ['Outside', 'Inside', 'yes', 'no', 'Mean G53', 'Span X', 'Span Y', 'Width X', 'mm/min',
        'Feeds and backoff', 'Probing inside %1 edge.', 'Probing outside %1 edge.'].sort());
});

test('UI source collection includes motion plans, feature names, states and confirmations', () => {
    const sources = collectSources();
    for (const source of ['Boss center', 'X valley center', 'Backoff', '°/min',
        'Start above a smooth part of the rod.', 'Touch opposite walls at this height. Find the midpoint.',
        'Unknown', 'Intermediate', 'Probing x ridge center.', 'Work zero set', 'X/Y rotation set',
        'boss center', 'interrupted', 'Rotary axis calibration', 'Width X', 'Width Y', 'Length Y',
        'Inside %1 corner', 'Outside %1 edge', 'The probing service could not complete the request (HTTP %1).'])
        assert.ok(sources.has(source), `Uncollected display source: ${source}`);
});

test('validation fails when one supported catalog loses a dynamic UI source', () => {
    const catalogs = loadCatalogs();
    delete catalogs.sv['Backoff'];
    assert.throws(() => validateCatalogs(catalogs), /sv: missing "Backoff" \(ui\/ReviewPlan\.js\)/);
});

test('validation reports a source missing from both supported catalogs', () => {
    const sources = new Map([['Probe', 'ui/example.qml'], ['Inside', 'ui/model.qml']]);
    assert.throws(() => validateCatalogs({zh_CN: {Probe:'探针'}, sv: {Probe:'Mätprob'}}, sources), error => {
        assert.match(error.message, /zh_CN.*"Inside".*ui\/model.qml/);
        assert.match(error.message, /sv.*"Inside".*ui\/model.qml/);
        return true;
    });
});

test('validation permits additional catalog entries and reordered or repeated placeholders', () => {
    const sources = new Map([['Move %1 to %2. Then %1.', 'ui/example.qml']]);
    assert.doesNotThrow(() => validateCatalogs({
        zh_CN: {'Move %1 to %2. Then %1.':'%2：移动 %1。然后 %1。'},
        sv: {'Move %1 to %2. Then %1.':'Till %2: flytta %1. Sedan %1.', Extra:'Extra'}
    }, sources));
});

test('validation rejects empty translations and changed placeholder numbers', () => {
    for (const translated of ['', '   ', 25, 'Flytta %1 till %3.']) {
        assert.throws(() => validateCatalogs({
            zh_CN: {'Move %1 to %2.':'将 %1 移动到 %2。'},
            sv: {'Move %1 to %2.':translated}
        }, new Map()), /sv: invalid translation or parameters/);
    }
});

test('validation requires a catalog for every supported language', () => {
    assert.throws(() => validateCatalogs({sv: {Probe:'Mätprob'}}, new Map([['Probe', 'ui/example.qml']])),
        /zh_CN: missing or invalid catalog/);
});

test('supported catalogs translate every required UI source with matching parameters', () => {
    validateCatalogs(loadCatalogs());
});
