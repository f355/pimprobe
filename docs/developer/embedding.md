# Embedding PIMProbe

## Application

`pimprobe-app` owns settings validation, reviews, running operations and result
actions. Construct `ProbeApp` with a device, settings, host actions and records.
The host supplies these interfaces:

- `ProbeDevice`: controller events, commands, current state, probe actuation
  and an exclusive machine guard. Keep that guard shared with other machine
  operations in the host.
- `SettingsStore`: load and save `ProbeSettings`.
- `HistoryStore`: append, read and clear `HistoryRecord` values.
- `Diagnostics`: record diagnostic events and clear diagnostic storage.
- `HostActions`: identify the installed software and export logs.

The standalone service implements these interfaces with its controller socket
and local files. Another host can call the application directly and provide its
own storage and device implementations.

Running an operation returns an `Operation` with typed progress, measurement,
result and error events. Dropping it requests cancellation. The machine guard
stays held until the operation has stopped. A storage error after measurement
can include the measured result in the error event.

## QML Page

Create `ProbePage` inside the host window and supply its `client`. Keep the page
alive when navigating away: hiding it preserves an active run and its results.
Destroying it cancels its requests.

Set `showHeader: false` when the host supplies the header. The page exposes:

- `headerControls`: a component containing the probe deploy switch.
- `pageTitle`, `busy` and `subpageOpen`: current navigation and operation state.
- `requestBack()`, `openHelp()` and `openWcs()`: header actions.
- `leaveRequested` and `alarmRequested`: requests for host navigation.
- `settingsContribution`: optional host controls for the Settings page.
- `updateAvailable`: shows a notification dot on the Settings tab.

Pass `uiFontFamily` and `monoFontFamily` for text. A `controls/Style.qml` object
sets colors, control sizes and layout spacing through the page's `style`
property. The controls share that style within the QML engine.

The `controls/I18n` singleton selects the interface language. Set
`systemLanguage` to `en`, `zh_CN` or `sv` to supply a host default. A saved
`language` setting takes precedence. Labels update when the language changes.

`ProbeWindow` supplies the standalone window, HTTP client, fonts, update page
and application exit handling.

## Client

The client exposes `state`, `error`, their change signals and `refresh()`.
`state` has the same fields as the standalone `/api/v1/state` response.

`state.coordinates.probe` and `state.coordinates.spindle` each contain
`machinePosition` and `workPosition` arrays in X/Y/Z/A order. Probe coordinates
use the fixed probe calibration; spindle coordinates include the last known
tool-length offset. A reference is `null` until its coordinates are available.

`request(operation, arguments, callback)` calls back with
`{ok: true, data: ...}` or `{ok: false, error: "..."}`. It returns an object with
an `abort()` method. Callbacks may complete synchronously.

`stream(operation, arguments, onEvent, onFinished)` delivers application
operation events and returns the same kind of abort handle. `onFinished`
receives an error string, or an empty string after normal completion.

The page uses these operation names:

| Operations | Purpose |
| --- | --- |
| `settings.get`, `settings.schema`, `settings.update` | Parameters and validation ranges |
| `probe.set`, `wcs.select` | Probe actuator and active work coordinates |
| `routine.review`, `routine.run` | Review and execute a measurement |
| `routine.zero`, `routine.return`, `routine.measured` | Actions on a completed result |
| `routine.wcs` | Select a WCS for a completed result |
| `rotary.review`, `rotary.run` | Review and execute rotary calibration or surface alignment |
| `rotary.zero`, `rotary.rotation` | Save rotary zeros or X/Y work-coordinate rotation |
| `rotary.wcs` | Select a WCS for a rotary result |
| `repeatability.run` | Repeated measurements |
| `repeatability.stop` | End the repeatability check after the current movement or touch finishes |
| `history.get`, `history.export`, `history.clear` | Measurement records and logs |
| `history.open` | Reopen a saved ordinary or rotary result |
| `history.wcs`, `history.zero`, `history.rotation` | Apply a reopened measurement |

Result WCS requests take `{id, wcs}` and return the updated measurement.
Review requests take a routine or rotary configuration and return
`{id, config, program, simulated}`. `config` contains all parameters used by
the plan. The confirmation page edits its options locally. Proceed sends the
review request using the current machine position, then runs its `{id}`.

`history.open` takes `{id}` and returns `{entry, result, canApply}`. Use
`entry.id` for its subsequent actions. Work-zero requests take `{id, offsets}`
with three X/Y/Z offsets; rotary zeros and rotation use `{id}`. History actions
return the updated result, and `canApply` indicates a completed measurement.

`client/HttpClient.qml` maps these calls to the standalone HTTP API. The update
page additionally uses `updates.check`, `updates.install` and `updates.status`.

## Assets

Include the `ui` QML files, `controls`, `client` and `i18n` directories. Generate
the translation catalog, `HelpPages.js` and help images with `buildHelp(outputDirectoryURL)` from
`dev/build-help.mjs`. The output directory contains the staged QML files.
Hosts supply their own fonts; the standalone build fetches its fonts with
`dev/fetch-fonts.mjs`.
