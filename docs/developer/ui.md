# Touchscreen UI

The standalone interface uses a dark palette and an 800 × 480 window. Design
each screen for the 4.9-inch touch panel, not for a desktop monitor.

## Browser Components

The React interface is in `web/src`. Use the controls in `controls.tsx`, CSS
grid or flex layouts, and the `--pp-` colors in `style.css`. `NumericProvider`
owns number editing; `KeypadSlot` replaces the left column while a field is
being edited. Fields save on Enter or when a result action accepts the edit.

`HelpProvider` reads `data-help-title` and `data-help` on controls and readout
groups. Help taps explain
the selected control; they do not run its action. The Guide reads the operator
Markdown and translation catalogs directly.

Run `npm test --prefix web` after browser changes. Inspect its 800 × 480
captures in `web/test-results`, including translated labels, keypads, reviews,
results and help. `./dev/preview.sh` runs the UI against the mock
controller for interactive checks.

## Layout

- Use CSS grid and flex layouts for rows, columns and grids.
- Leave 16 px around screen content and 12 px between controls.
- Use `PageHeader` for a 64 px header with Back on the left and status on the
  right. Keep the same header on review, results, history and utility screens.
- Probing pages have a 352 px diagram, measurement or keypad column on the left and aligned
  labels and inputs on the right.
- Keep result actions in a bottom row outside the scrolling content. Show the
  command log separately from the measurements.
- Use `ConfirmDialog` for confirmations inside the window. Its actions are
  equally wide, 80 px high, and inset from the dialog edges.

## Controls and text

Use the controls in `web/src/controls.tsx` and colors from `style.css`.
A host can set the `--pp-` CSS variables when embedding the component.

Buttons and number fields are normally 56 px high; icon controls are at least
48 × 48 px. Use 20 px for ordinary text, 24 px for page headings and number
fields, and 28 px for measured coordinates. Secondary text is 16–18 px.
Coordinates and command logs use a monospaced font.

Give command buttons clear verbs. Use captions on diagrams when the drawing
alone could be mistaken for another operation. Keep labels next to their
inputs and use the same input width within a form.

## Motion previews

`OperationReview` shows a looping motion illustration beside the distances and
feeds. Its number fields edit a local draft. Proceed accepts the active field.
`ProbeFlow` then sends the options to the application, which
prepares the moves from the current machine position. The pages run the returned
token. Saved settings are unchanged.

`MotionIllustration` plays PNG frame sheets from `web/assets/animations`. The scenes
use Three.js in `dev/animations`; the browser plays the rendered frames. Each loop ends
with a pulsing result in the axis colours while the probe and path fade out.
Rendering requires Node.js and Chrome:

```sh
npm ci --prefix dev/animations
npm run render --prefix dev/animations
node dev/animations/gallery.mjs
```

The gallery is saved as `build/ui-review/motion-loops.html`. The renderer checks
for movement, a pulsing result, visible pixels and probe framing. Each frame
sheet fits inside a 2048 × 2048 texture.

## Checking screens

Run `npm test --prefix web` after changing shared controls or layouts. The screen
tests cover tabs, keypads, dialogs, review and result pages, history, updates,
and disconnected or failed operations. They check touch targets and text
fit, and save 800 × 480 captures in `web/test-results`.

Inspect those captures as well as the test results. Check alignment, readable
labels, scrolling, visible actions and whether diagrams explain the operation.

## Languages and help

The question-mark button toggles `HelpProvider`. It outlines visible controls and
intercepts their taps. Use `helpProps` for related readouts or tab buttons.
`Button`, `NumberField` and `Switch` accept `helpTitle` and `help`.
Use one or two sentences about what the control changes.

Explanations stay inside the screen and beside the selected control when space
allows. Guide opens the Markdown pages; Back returns to control explanations.
The Guide button remains available while help is active.

Use `t` from `useTranslation` for labels and complete messages, with `%1`, `%2` placeholders
for values. Translate service messages when displaying them. Keep
commands, measurement values and saved records in their original form.

The Chinese and Swedish catalogs are in `web/src/locales`. Translation tests
check their parameters and formatted service messages. The screen
tests check all three languages at 800 × 480.

Operator Markdown in `docs`, `docs/zh_CN` and `docs/sv` also supplies the help
pages. Keep the introduction short. `##` headings become expandable sections;
illustrations scale to the available width. Use short paragraphs and lists
instead of wide tables.

Explain where the ball starts, what each input changes, and where it finishes.
Name the control or picture the operator needs. Keep each tab's help about its
own routines; place links to other guide pages inside `guide-only` blocks.
Use numbered moves in diagrams only when the text identifies those moves.
