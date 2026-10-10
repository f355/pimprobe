# Probing guide

Choose a measurement:

- [Outside](outside.md): stock edge, outside corner or top surface.
- [Inside](inside.md): pocket wall, inside corner or bottom surface.
- [Center](center.md): boss, block, hole, pocket, ridge or valley center.
- [Rotary](rotary.md): rotary axis center or a face's angle.

The crosshair on each button marks where to start the **probe ball**, not the spindle. Arrows point toward the surfaces it will touch; the green dot marks the result.

<!-- guide-only -->
![Outside probing controls](images/outside.png)
<!-- /guide-only -->

## Starting a measurement

Select a picture to open the confirmation screen. Its animation shows the route. Compare that route with your stock and clamps, including the probe shaft's clearance.

You can change distances and feeds here for this run only. **Proceed** starts from the ball's position at that moment, so you can still jog before pressing it. **Cancel** returns without moving.

<!-- guide-only -->
![Confirmation screen with motion animation and inputs](images/review.png)
<!-- /guide-only -->

The probe touches each surface twice: a coarse search, backoff, then a slower fine touch. The fine touch supplies the measurement.

[Results and setting work zero](results.md)

## Coordinates and controls

- Large readouts use the selected work coordinate system; small readouts use machine coordinates, G53.
- **Probe / Tool** selects the ball or tool-tip readout. Tool uses the last known tool length.
- **G54–G59** selects a work coordinate system. Each stores its own zero.
- The switch beside the tabs extends or retracts the probe independently of measurements.
- **?** explains the current tab.

## Contact and failures

Sideways and downward positioning moves stop on ball contact. Jogging and ordinary upward moves do not. A shaft collision may never trigger the ball.

If the first search misses, the measurement fails. If the fine touch misses, the firmware can alarm and return you to the machine's main screen. Check the search distance and starting position before retrying.

## Other pages

- [Settings](settings.md): ball diameter, backoff, feeds and updates.
- [Editing numbers](editing.md)
- [History and logs](history.md)
- [Repeatability check](repeatability.md)
