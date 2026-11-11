# Embedding PIMProbe

## Rust application

Use `pimprobe-app::ProbeApp` for probing operations, calibration, settings and
history without running an HTTP service. The host supplies:

- `ProbeDevice`: machine state, commands, events, probe actuation and an
  exclusive machine guard shared with the host's other machine operations.
- `SettingsStore`: load and save probing preferences.
- `HistoryStore`: append, read and clear measurement and calibration records.
- `Diagnostics`: record and clear diagnostic logs.
- `HostActions`: identify the installed software and export logs.

`SettingsStore` and `HistoryStore` must report write errors and make saved changes
durable before returning success. The machine may lose power without shutting down.

Keep the machine guard until the operation has stopped, including after
cancellation. If the host batches commands, implement `finish_action` to finish
that batch before publishing a result. Dropping an `Operation` requests
cancellation; it does not mean the machine has already stopped.

A storage error after successful probing can carry the measured result. Preserve
that result and display the storage warning rather than discarding the measurement.

See the public traits in [`crates/probe-app/src`](../../crates/probe-app/src)
for method signatures. The standalone service provides implementations using
the controller bridge and local files.

## React component

`web/src/index.ts` exports `ProbeApp`, which includes its connection provider.
Give its container a width and height; the touchscreen layout uses 800 × 480.
Import `web/src/style.css` if the build system does not follow CSS imports.

Supply a `ProbeClient` through the `client` prop to use the host's application API.
Without one, the component uses the standalone HTTP API at `/api/v1`.

For an application with several pages, keep one provider above navigation:

```tsx
<ProbeProvider client={client} connected={connected}>
  <ApplicationPages>
    <ProbeWorkspace updates={false} onLeave={closeProbe} />
  </ApplicationPages>
</ProbeProvider>
```

Set `connected` from the host connection state. Use `useProbe()` for the shared
machine state, settings and save/reload actions. Call `reloadSettings` after
changing probing settings outside the component.

Keep the workspace mounted but hidden between visits to preserve its selected
tab and measurement result. Keep it mounted during an operation, including
while showing help.

## Client contract

A `ProbeClient` provides:

```ts
request<T>(operation: string, arguments?: object, signal?: AbortSignal): Promise<T>;
stream<T>(operation: string, arguments: object,
          onEvent: (event: T) => void, signal?: AbortSignal): Promise<void>;
```

Requests return a result or throw an error. Streams deliver progress and terminal
events before resolving. An aborted signal requests cancellation.

Use the operation names and HTTP mappings in
[`client.ts`](../../web/src/client.ts), and the measurement and state types in
[`types.ts`](../../web/src/types.ts).

`state.coordinates.probe` and `state.coordinates.spindle` provide machine and
work positions in X/Y/Z/A order. Probe positions use the saved probe calibration;
tool positions use active tool-length compensation. See
[Coordinates](coordinates.md) for their Z references. Either position can be
`null` until the required machine data is available.

## Navigation and host controls

- `onLeave` closes probing after the probe-retraction choice.
- `onAlarm` lets the host show its alarm screen; without it, alarms use `onLeave`.
- `defaultLanguage` supplies the host language until the operator saves a choice.
- `updates={false}` hides software updates when the host manages them.
- `renderHeader` supplies the host's header, including coordinate selection,
  navigation and help actions.
- `renderFooter` adds host navigation below the page.

Use `leave(destination)` from footer props, or a `ProbeNavigation` reference
supplied through `navigationRef`, when leaving probing. It handles probe
retraction and blocks leaving while an operation is running.

## Assets

The build includes translation catalogs, operator Markdown, illustrations,
animation sheets and fonts. Hosts can set `--pp-` CSS variables to match their
palette. Run `node dev/fetch-fonts.mjs` before building to fetch the pinned fonts.
