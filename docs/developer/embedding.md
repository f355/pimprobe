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

## React Component

`web/src/ProbeApp.tsx` exports `ProbeApp`. Give its containing element a width
and height; the touchscreen layout uses 800 × 480. Import `web/src/style.css`
when the build system does not follow CSS imports automatically.

The optional `client` prop supplies the application operations. Without it,
the component uses the HTTP API at `/api/v1`. A supplied `ProbeClient` implements:

```ts
request<T>(operation: string, arguments?: object, signal?: AbortSignal): Promise<T>;
stream<T>(operation: string, arguments: object,
          onEvent: (event: T) => void, signal?: AbortSignal): Promise<void>;
```

Requests return the operation's result or throw an error. Streams deliver
progress and terminal events, then resolve. An aborted signal cancels the
request or stream. `state` supplies the current machine snapshot; the component
refreshes it every 250 ms. The `onLeave` prop handles the Back action after the
probe-retraction choice. `onAlarm` handles a controller alarm during probing;
it defaults to `onLeave` when no separate alarm handler is supplied.

Keep the component mounted during operations. The Guide hides the underlying
page while keeping its requests and result state alive. CSS variables beginning
with `--pp-` set the palette and column width. Controls, keypad and help styles
are limited to the component's classes.

The Vite build embeds the operator Markdown and translation catalogs, and
copies illustrations, animation sheets and fonts into `dist/media`.
`defaultLanguage` supplies the host's language until a saved language is selected.

## Client

The `state` request returns the same fields as the standalone `/api/v1/state`
response.

`state.coordinates.probe` and `state.coordinates.spindle` each contain
`machinePosition` and `workPosition` arrays in X/Y/Z/A order. Probe coordinates
use the fixed probe calibration; spindle coordinates include the last known
tool-length offset. A reference is `null` until its coordinates are available.

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

`HttpClient` maps these calls to the standalone HTTP API. The update
page additionally uses `updates.check`, `updates.install` and `updates.status`.

## Assets

The Vite build includes translation catalogs from `web/src/locales` and guide
pages from `docs`. It copies `web/assets` and guide illustrations into
`web/dist/media`. `dev/fetch-fonts.mjs` fetches the pinned fonts before building.
