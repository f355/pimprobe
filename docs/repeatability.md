# Probe repeatability

Open **Settings → Utilities → Probe repeatability** to measure the same reference surfaces several times.

## Prepare

Home the machine with the L bracket installed at the bottom left of the table. Clear the path to the bracket and leave room for the probe to extend, then confirm **Fixture is ready**. The selected work coordinate system can stay as it is.

The routine uses approximate machine-coordinate references for the bracket walls (G53 X-232.5 and Y-204.3) and bed (Z-62.9), adjusted for the machine's probe calibration. The measuring point puts the probe tip about 15 mm from each wall and 5 mm above the bed. With homing off, X/Y travel happens at the current height before Z lowers.

## Options

<!-- guide-only -->
![Repeatability options](images/repeatability-options.png)
<!-- /guide-only -->

- **Axes:** choose X, Y and/or Z. All three are selected initially.
- **Repetitions:** how many times to measure each selected surface. The default is 5.
- **Home each time:** home before every repetition, including the first. This includes homing variation and turns on retraction. Starts off.
- **Retract each time:** retract and extend the probe between repetitions. This includes deployment variation. Starts on.

The check uses the ball diameter, feeds and retract distance from Settings.

## Running

Press **Start check**. With homing off, an already-extended probe stays extended for the initial approach. A retracted probe extends after reaching the measuring point.

With homing selected, an extended probe first moves to the measuring X/Y at its current height and retracts there. The machine then rapids to machine Z0, then machine X-20 Y-5, and homes. This leaves the final approach to the X/Y switches to the slow homing cycle. It extends the probe at home and returns to the measuring point using contact-guarded moves. Between repetitions, retraction happens at the measuring point before any Z lift.

The probe moves to the bracket in X/Y, then lowers toward the bed. X probes toward X-, Y toward Y-, and Z toward the bed. The coarse search covers 20 mm in X/Y or 10 mm in Z. Each selected axis gets a fine touch and returns to the measuring point before the next axis. After the last repetition, the probe stays extended at the measuring point.

If a movement or measurement fails, the check stops and keeps the readings collected so far.

## Results

<!-- guide-only -->
![Repeatability results](images/repeatability-results.png)
<!-- /guide-only -->

While running, each row shows the measured G53 coordinates in millimeters. When the check ends, the rows show each measurement's deviation from that axis's mean. A positive value is above the mean; a negative value is below it.

- **Mean G53:** the average measured coordinate.
- **Median:** the middle reading after sorting; for an even number, the average of the middle two.
- **Std dev:** sample standard deviation, showing how much the readings scatter. It needs at least two readings.
- **Range:** highest reading minus lowest reading.

The summary updates after every measurement, using the readings collected so far for each axis. Mean and median are G53 coordinates. Homing and retraction choices are shown below the results.
