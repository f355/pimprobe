# Repeatability check

Open **Settings → Utilities → Probe repeatability**. Fit the tall L bracket at the lower-left corner of the bed and home the machine. The routine uses machine coordinates to reach it, so it does not need a work zero.

Without a home-area move, the first X/Y approach happens at the current Z. Start high enough to clear the bed fixtures.

## Options

- **Axes:** choose which bracket walls and/or bed surface to measure.
- **Repetitions:** readings per axis; default 5.
- **Retract each time:** include variation from retracting and extending the probe.
- **Jog home each time:** retract, travel to G53 Z0 then X−20 Y−5, and return before measuring.
- **Home each time:** also re-home on each visit to the home area.

The two home options include retraction. Without them, retraction happens at the bracket's measuring position.

The ball approaches 15 mm from each wall and 5 mm above the bed, then touches the selected surfaces. **Stop** ends the test after the current move or touch finishes. The last readings are kept.

## Read the results

Rows show G53 readings during the test, then deviations from each axis's mean afterward. The summary updates after every touch.

- **Mean G53:** average coordinate.
- **Median:** middle value after sorting, or the average of the middle two for an even count.
- **Std dev:** sample standard deviation; needs at least two readings.
- **Range:** highest minus lowest reading.

<!-- guide-only -->
![Repeatability readings and statistics](images/repeatability-results.png)
<!-- /guide-only -->
