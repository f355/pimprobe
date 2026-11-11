# Touchscreen UI

Design for the 4.9-inch, 800 × 480 touch panel. Reuse the controls and layout
defaults in `web/src` rather than copying their CSS into individual pages.

## Layout and controls

- Use grid and flex layouts. Allow labels to wrap without clipping controls.
- Use the shared buttons, switches, dialogs and number fields.
- Keep related inputs aligned and leave space between separate actions.
- Keep the main action visible while details scroll.
- Use captions when a diagram alone could be mistaken for another operation.
- Use `PageHeader` so help stays available and a host can supply its own header.
- Reserve dialogs for confirmation or short choices; give longer tasks a page.

`NumericProvider` manages number editing. Use `NumberField` for inputs and
`KeypadSlot` where the keypad should replace other content.

Scope styles to the component. Hosts can change the `--pp-` CSS variables
without affecting the rest of their application.

## Help and language

Use US English spelling and phrasing in source strings and English documentation.
The English locale is `en_US` (`en-US` in HTML language attributes).

Use `t` from `useTranslation` for complete messages, with `%1`, `%2`
placeholders for values. Keep commands, measurements and saved records in
their original form. When changing an English message, update its keys in the
Chinese and Swedish catalogs under `web/src/locales`.

Use the controls' `help` props or `helpProps` for a group of related readings.
Explain what the control changes in one or two short sentences. Help must not
activate controls or edit values, including when explaining a disabled control.
F1 opens help for the focused control.

The on-screen Guide uses the operator Markdown in `docs`, `docs/zh_CN` and
`docs/sv`. Keep the introduction short. Each `##` heading becomes an expandable
section. Use short paragraphs, numbered steps and illustrations rather than
wide tables.

Explain where the ball starts, what each input means and where the ball ends.
Name the actual controls. Keep developer details out of the operator guide.

## Preview and checks

`./dev/preview.sh` runs an interactive preview against a simulated controller.
Run `npm test --prefix web` after UI or help changes. Inspect the captures in
`web/test-results`, especially the 800 × 480 screens in all three languages.
Check readable labels, alignment, touch targets, scrolling and visible actions.

Motion illustrations are generated from the Three.js scenes in
`dev/animations`. Rendering requires Node.js and Chrome:

```sh
npm ci --prefix dev/animations
npm run render --prefix dev/animations
node dev/animations/gallery.mjs
```

Inspect the gallery at `build/ui-review/motion-loops.html` after changing a scene.
