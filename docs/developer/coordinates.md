# Z Coordinates

The controller reports carriage position in `MPos` and tool position in `WPos`.
For Z, their relationship is:

```text
WPos Z = MPos Z − work origin Z − G92 Z − active TLO
```

`$#` reports the active tool-length offset as `[TLO:…]`. `$201` is the
stored tool length; `G43.1` and `G49` can change the active offset without
changing that setting. The last number in `[PROBE:…]` is `$202`, the stored
ETS contact, rather than the active TLO.

Probing reads `$#` before capturing its starting coordinates and again before
setting work zero. Each read waits for the selected work origin, G92, TLO and
the final stored probe report. A missing response stops the operation.

## Probe Calibration

`$35` relates the probe's Z contact to the stored ETS contact. To convert a raw
probe contact into a work origin, use:

```text
surface Z = raw contact Z + $202 − active TLO − $35
```

The controller adapter supplies the core with a Z offset of
`$35 − $202 + active TLO`. The core subtracts that offset from carriage Z for
both measurements and movement calculations. The probe DRO uses the same
conversion.

Tool measurement changes `$201` and `$202` by the same amount, keeping their
difference constant. `G49` cancels the active TLO without changing `$202`, so
the conversion needs both values even when the active TLO is zero.

## Setting Work Zero

`G10 L20` takes the desired current work coordinate, not a machine coordinate.
To put Z zero at a measured machine coordinate, the command needs:

```text
G10 Z value = current MPos Z − measured surface Z − active TLO
```

For example, with carriage Z −20, surface Z +24 and TLO −60, the command is
`G10 L20 P1 Z16`. G54's origin becomes +24, but `G54 Z0` targets carriage
Z −36 because the controller adds the −60 TLO. A positive work origin can
therefore be valid.

For a raw probe contact of −26, `$35` of −50 and `$202` of −60, cancelling
the TLO changes the measured surface to −36. The command is still
`G10 L20 P1 Z16`, and `G54 Z0` still targets carriage Z −36.
