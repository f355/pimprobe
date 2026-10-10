# Touchscreen UI

The standalone interface uses a dark palette and an 800 × 480 window. Design
each screen for the 4.9-inch touch panel, not for a desktop monitor.

## Layout

- Use Qt Quick Layouts for rows, columns and grids.
- Leave 16 px around screen content and 12 px between controls.
- Use `PageHeader` for a 64 px header with Back on the left and status on the
  right. Keep the same header on review, results, history and utility screens.
- Probing pages have a 352 px diagram or keypad column on the left and aligned
  labels and inputs on the right.
- Keep result actions in a bottom row outside the scrolling content. Show the
  command log separately from the measurements.
- Use `TouchDialog` for confirmations inside the window. Its actions are
  equally wide, 80 px high, and inset from the dialog edges.

## Controls and text

Use the controls in `ui/controls` and colors from `Theme`. `Style` supplies the
palette, spacing and sizes. A host can supply its own `Style` when embedding
the page.

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
`ProbeFlow` and `RotaryFlow` then send the options to the application, which
prepares the moves from the current machine position. The pages run the returned
token. Saved settings are unchanged.

`MotionIllustration` plays PNG frame sheets from `ui/animations`. The scenes
use Three.js in `dev/animations`; QML plays the rendered frames. Rendering
requires Node.js and Chrome:

```sh
npm ci --prefix dev/animations
npm run render --prefix dev/animations
node dev/animations/gallery.mjs
```

The gallery is saved as `build/ui-review/motion-loops.html`. The renderer checks
for movement, visible pixels and probe framing. Each frame sheet fits inside a
2048 × 2048 texture.

## Checking screens

Run `./dev/test-ui.sh` after changing shared controls or layouts. The screen
tests cover tabs, keypads, dialogs, review and result pages, history, updates,
and disconnected or failed operations. They check touch targets and text
fit, and save numbered 800 × 480 captures in `build/ui-captures`.

Inspect those captures as well as the test results. Check alignment, readable
labels, scrolling, visible actions and whether diagrams explain the operation.

## Languages and help

Use `I18n.tr` for labels and complete messages, with `%1`, `%2` placeholders
for values. Translate service messages when displaying them. Keep
commands, measurement values and saved records in their original form.

The Chinese and Swedish catalogs are in `ui/i18n`. `dev/build-i18n.mjs`
checks their parameters and generates the catalog used by QML. The screen
tests check all three languages at 800 × 480.

Operator Markdown in `docs`, `docs/zh_CN` and `docs/sv` also supplies the help
pages. Keep the introduction short. `##` headings become expandable sections;
illustrations scale to the available width. Use short paragraphs and lists
instead of wide tables.

Explain where the ball starts, what each input changes, and where it finishes.
Name the control or picture the operator needs. Keep each tab's help about its
own routines; place links to other guide pages inside `guide-only` blocks.
Use numbered moves in diagrams only when the text identifies those moves.
