# Probing guide

Start with [Calibrate the probe](calibration.md) before using the probe to set
a workpiece's zero.

Choose a measurement:

- [Outside](outside.md): stock edge, outside corner or top surface.
- [Inside](inside.md): pocket wall, inside corner or bottom surface.
- [Center](center.md): boss, block, hole, pocket, ridge or valley center.
- [Angle](angle.md): face angle in X/Y or Z slope along X or Y.
- [Rotary](rotary.md): rotary axis center or a face's angle.

The crosshair marks the **probe ball's** starting position. White arrows show movement; crossed circles show downward touches in a top view.

References use the axis colors: **X red, Y green, Z blue**. A dot marks a possible zero point, a line or shaded area marks a zero plane, and an angled line shows the measured direction. These marks show what you can set from the result; measuring alone leaves the WCS unchanged.

<!-- guide-only -->
![Outside probing controls](images/outside.png)
<!-- /guide-only -->

## Starting a measurement

Select a picture to open the confirmation screen. It shows the route. Compare that route with your stock and clamps, including the probe shaft's clearance.

You can change distances and feeds here for this run only. **Proceed** starts from the ball's position at that moment, so you can still jog before pressing it. **Cancel** returns without moving.

<!-- guide-only -->
![Confirmation screen with motion animation and inputs](images/review.png)
<!-- /guide-only -->

The probe touches each surface twice: a coarse search, backoff, then a slower fine touch. The fine touch supplies the measurement.

[Results and setting work zero](results.md)

## Coordinates and controls

- Large readouts use the selected work coordinate system; small readouts use machine coordinates, G53.
- Select **Probe** for the probe ball or **Tool** for the tool tip. Probe uses
  its own calibration; Tool uses the machine's active tool-length compensation.
- **G54–G59** selects a work coordinate system. Each stores its own zero.

Edges, corners and centers follow the selected work-coordinate axes, including its X/Y rotation. Face-angle and rotary measurements follow the machine axes.

- Tap **Retracted / Extended** beside the tabs to extend or retract the probe independently of measurements. The highlighted label shows its current state.
- **?** highlights the controls. Tap one for an explanation, or **Guide** for these pages. **×** leaves help.

## Contact and failures

Sideways and downward positioning moves stop on ball contact. Jogging and ordinary upward moves do not. A shaft collision may never trigger the ball.

If a search misses the surface, the measurement stops with an error. A
controller alarm may need clearing in the machine controls before retrying.
Make sure the ball can reach the intended surface in the chosen direction
within the search distance.

## Other pages

- [Settings](settings.md): ball diameter, backoff, feeds and updates.
- [Editing numbers](editing.md)
- [History and logs](history.md)
- [Repeatability check](repeatability.md)
