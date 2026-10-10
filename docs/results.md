# Results and work zero

The large coordinates are the measured surface or center in **G53**. The smaller coordinates show the same point in the selected **G54–G59**. Probe calibration and ball-radius compensation are already applied.

For centered features, **Size** includes ball compensation; **Raw span** is the distance between the fine contacts before compensation. Edge and Z results show the uncompensated machine coordinates under **Contacts**.

<!-- guide-only -->
![Measurement results and zero offsets](images/result.png)
<!-- /guide-only -->

## Set work zero

Choose the WCS beside **Work zero**, enter offsets if needed, then press **Set Work Zero**. Only measured axes change. The machine stays still.

Each new zero is **measured G53 coordinate + Offset**. For example, if Z measures 25 and you enter −2, Z0 is at G53 Z23.

You can adjust offsets and set zero again, or select another WCS and set it from the same measurement.

## Positioning buttons

- **Return to start:** move X/Y to the saved start, then Z. Available for Outside and Center results.
- **Move to measured XY:** on Inside wall/corner results, raise by **Safe Z lift**, then move the ball above the measured point. The lift is relative to current Z.
- **Close:** leave the result without moving.

These destinations position the **probe ball**, not the spindle. Z-only routines have already returned to their start.

## Saved measurements

Open **Settings → Utilities → Probe history**, select a measurement, then **Open result**. You can set work zero from it later, provided the stock and machine reference are still the same.
