# Calibrate the probe

The milled-block procedure below calibrates the ball diameter and all three offsets.
You can instead use a gauge block or pin for the diameter, or a dial test
indicator for the X/Y offsets.

## What calibration measures

### Ball diameter

The probe does not signal contact the instant its ball touches a surface.
The machine moves a little farther while the stylus deflects and the probe's
internal mechanism moves enough to operate its electrical contacts.

PIMProbe accounts for this extra movement by using an **effective ball
diameter**, also called the electrical ball diameter. It is smaller than the
physical ball. For example, a ball with a physical diameter of 2 mm might need
a setting of 1.83 mm. Calibration finds this value by comparing a probed width
with a width measured independently, such as with a micrometer.

PIMProbe uses half this diameter to calculate where a touched wall lies. It
also uses the diameter when calculating a width between two walls. Entering
the physical ball diameter instead would put those walls in the wrong places.

### X/Y offsets

The probe is mounted beside the spindle. PIMProbe needs the X and Y distances
between them to put the spindle over a point found by the probe.

To measure these offsets, first locate the center of a block, boss, hole or pocket
with the spindle, then find the same center with the probe. Touching both
opposite sides gives their midpoint. The ball radius affects the two sides
equally in opposite directions, so it does not change that midpoint. You can
therefore calibrate X/Y offsets without knowing the ball diameter.

### Z offset

Z calibration makes a surface touched by the probe agree with the height of
that same surface cut by a tool whose length has been measured.

## Make a block for calibration

This example uses a rectangular raised block. The same method works with any
X/Y center feature on **Center**: block, boss, hole or pocket, provided you
have instruments to measure its finished dimensions accurately.

1. Clamp the stock to the worktable. Keep the clamps away from the four sides
   you will measure.
2. Mill a block with its center at X0/Y0 in a work coordinate system of your
   choice and its sides parallel to the X and Y axes. For Z calibration,
   finish its top at Z0 using a tool whose length has been measured with the
   tool setter. Leave the stock clamped and the work zero unchanged after milling.
3. Measure the finished width along X and along Y with a micrometer. Write
   down both measurements.

**End mills often cut under their nominal diameter.** The finished block may
therefore differ from the programmed size. Use the measured block dimensions
for calibration, not the programmed dimensions or the size marked on the tool.

## Probe the block

1. Stop the spindle. Remove chips from the top and four sides of the block.
   Remove any burrs from the edges you will touch.
2. Open PIMProbe and select the same work coordinate system used to mill the
   block. Extend the probe with the **Retracted / Extended** control and
   clean the ball if needed.
3. Jog until the ball is roughly above the center of the block. Raise it so
   the whole ball can pass over the top without touching it.
4. Open **Center** and select **Block**.
5. Set **X search distance** and **Y search distance**. Each is the distance
   traveled from the starting point to either side, before lowering the
   ball. It must put the whole ball beyond the edge of the block.
6. Set **Depth** to lower the ball from its starting height to the vertical
   sides. For example, with the bottom of the ball 5 mm above the block,
   a depth of 8 mm lowers it to 3 mm below the top. The ball must touch the
   sides, not the top edge, and the stem must not hit the block.
7. On the review page, press **Proceed** to measure.
   The probe touches both X sides and both Y sides, then returns above the
   measured center.

For example, a 30 mm wide by 20 mm long block, started near its center, could
use an X search distance of 20 mm and a Y search distance of 15 mm. Check that
these outward moves and the subsequent lowering moves cannot hit the stock
or its clamps.

## Save the block calibration

Keep the result open. **Do not press Set Work Zero:** X0/Y0 already marks the
center used when milling. Replacing it with the uncalibrated probe reading
would remove the comparison needed to calculate the offsets.

1. Press **Calibrate probe...**, then select **Ball diameter**.
2. Select **X+Y**. Enter the micrometer measurements in **Known X size** and
   **Known Y size**.
3. The page calculates a diameter from each width. **New** is the average of
   those two diameters; this is the single value that will be saved. Select
   **X** or **Y** instead if you want to use only one measured width.
4. Press **Save ball diameter**. This returns to the measurement result.
5. Press **Calibrate probe...** again and select **X/Y offsets**.
6. Select **X+Y**, then press **Save X/Y offsets**. These offsets make the
   probed center agree with the unchanged X0/Y0 used for milling.
7. Close the result and probe the block again. Its measured sizes should
   agree with the micrometer measurements, and its center should be close
   to X0/Y0.
8. Close that result. Jog the ball above a flat part of the block's top,
   a few millimeters above it.
9. Choose **Center → Z**. Set **Depth** greater than the gap between the ball
   and the top; this is the maximum downward distance it will search.
10. On the review page, press **Proceed**. After the touch, the probe rises
    to its starting height.
11. Without pressing **Set Work Zero**, press **Calibrate probe...**, then
    **Save Z offset**. The block's top was milled at Z0, so this sets the
    probe to report that same height.
12. Probe the top again. It should read close to Z0.

## Use a gauge block or pin for ball diameter

You do not need to know where the gauge is on the table. You only need its
width or diameter. This procedure alone does not calibrate X/Y offsets.

### Gauge block

1. Clamp a gauge block with both faces of its known dimension accessible to
   the probe. Clean those two faces.
2. Align the block so those faces are perpendicular to X or Y. An indicator
   swept along a face can check the alignment: the reading should stay
   constant. A tilted block would give a larger width along the probing axis.
3. Extend the probe and jog the ball above the middle of the block, high
   enough to pass over it.
4. On **Center**, choose **X ridge** if measuring across X, or **Y ridge** if
   measuring across Y. Set the search distance and depth as described in
   **Probe the block**, so the ball lowers beyond each side and touches the
   two measuring faces.
5. On the review page, press **Proceed**.
6. On the result, press **Calibrate probe... → Ball diameter**. Enter the
   gauge block's known dimension and press **Save ball diameter**.
7. Measure the gauge block again to check that the displayed width agrees.

### Gauge pin

1. Clamp a gauge pin upright, parallel to Z, with enough of its cylindrical
   side exposed for the ball to touch all around it. Clean the pin.
2. Extend the probe and jog the ball above the pin, roughly over its center.
   On **Center**, choose **Boss**. Set search distances that put the whole
   ball beyond the pin, and a depth that reaches its cylindrical side.
3. On the review page, press **Proceed**. After measuring,
   the ball finishes above the pin's measured center.
4. Close the result and run **Boss** again from this position. This second
   measurement crosses through the center; an off-center first measurement
   can read less than the full diameter.
5. Press **Calibrate probe... → Ball diameter**. Select **X+Y** and enter the
   pin's known diameter in both size fields. Press **Save ball diameter**.
6. Measure again to check that both displayed diameters agree with the pin.

## Use an indicator for X/Y offsets

Use a clamped part with a round hole or round boss. Its diameter does not
need to be known. A dial test indicator in the spindle establishes where
its center is before the probe measures it.

1. Fit the indicator in the spindle. With the spindle motor stopped, rotate
   the spindle by hand to sweep the hole wall or the outside of the boss.
2. Adjust X and Y until the indicator gives the same reading around the
   circle. The spindle is now centered on the hole or boss.
3. Use the machine's work-coordinate controls to set X and Y to zero in the
   chosen work coordinate system. Do not move the part after this.
4. Remove the indicator and extend the probe. Select that same work
   coordinate system in PIMProbe.
5. For a **Hole**, jog the ball inside the hole at the height to be measured.
   Set X/Y search distances long enough to reach both walls from the start.
   The ball crosses the hole at this height; **Depth** is not used.
6. For a **Boss**, jog the ball above its center and set search distances and
   depth as described for the gauge pin above.
7. Choose **Center → Hole** or **Center → Boss** as appropriate. On the review
   page, press **Proceed**.
8. Without pressing **Set Work Zero**, open
   **Calibrate probe... → X/Y offsets**. Select **X+Y** and press
   **Save X/Y offsets**.
9. Probe again. The measured center should be close to X0/Y0.

## When to recalibrate

Repeat calibration after replacing the stylus (the stem and ball), adjusting
the probe's mounting, or a collision that may have bent or shifted the stylus.

## Restore a previous calibration

1. Open **Settings → Probe history**.
2. Select the **Probe calibration** entry for the change you want to undo.
   **Previous** shows the values before that change; **Saved** shows the
   values written by it.
3. Press **Restore previous calibration**, then confirm. This restores the
   ball diameter and all three offsets shown under **Previous**.

Restoring does not move the machine or change a work zero. If you set a
workpiece's zero using an incorrect calibration, probe that workpiece again.
The restoration gets its own history entry, so you can undo it in the same way.
