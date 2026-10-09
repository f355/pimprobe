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

## Checking screens

Run `./dev/test-ui.sh` after changing shared controls or layouts. The screen
tests cover tabs, keypads, dialogs, review and result pages, history, updates,
and disconnected or failed operations. They check touch targets and text
fit, and save numbered 800 × 480 captures in `build/ui-captures`.

Inspect those captures as well as the test results. Check alignment, readable
labels, scrolling, visible actions and whether diagrams explain the operation.
