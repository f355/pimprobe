# Coordinates

## Carriage, tool and probe

The controller's `MPos` is carriage position. Tool and probe readings refer to
different points and must not share the active tool's length correction.

Tool Z uses the currently active tool-length offset:

```text
tool Z = carriage Z − active TLO
```

Probe X/Y use the saved `$33`/`$34` corrections. Probe Z uses the saved
calibration and tool reference described below. Changing or canceling active
tool compensation must not change the probe reading at a fixed carriage position.

## Saved probe Z reference

`$201` is the saved tool-length offset, and `$202` is the ETS contact saved
with it. Their difference defines the firmware's tool-length reference:

```text
reference Z = $202 − $201
probe Z correction = $35 − reference Z
probe Z = carriage Z − probe Z correction
```

Ordinary tool measurement changes `$201` and `$202` by the same amount, leaving
the reference unchanged. It remains usable after the tool is unloaded or active
compensation is canceled. Neither `$201` nor `$202` alone is the probe correction.

Use the saved pair together. Do not replace `$202` with the latest probe-contact
report. Probe measurements, readouts and moves to a probe-coordinate target
must use the same correction.

## Work coordinates and work zero

The controller's work Z includes its active tool compensation:

```text
WPos Z = MPos Z − work origin Z − G92 Z − active TLO
probe work Z = WPos Z + active TLO − probe Z correction
```

Adding active TLO back removes the tool correction already present in `WPos`;
it does not make the probe depend on that tool.

A measured surface already includes probe calibration. Do not apply it again
when setting work zero. `G10 L20` takes the desired current work coordinate,
not the new origin. For a desired surface zero, including any operator offset:

```text
G10 L20 Z value = current carriage Z − desired surface zero Z − active TLO
```

## X/Y rotation

Edges, corners and centers follow the selected work-coordinate axes, including
their X/Y rotation. Face-angle and rotary measurements follow the machine axes.

Keep recorded contact and result coordinates in the unrotated machine frame.
Changing WCS changes how those points are expressed, not where they were measured.
