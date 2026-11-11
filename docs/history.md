# History and logs

Open **Settings → Utilities → Probe history**. Select a run to see its measurements, inputs, commands and any error.

**Open result** lets you use a saved measurement to set work zero. The stock and machine reference must still match that measurement.

## Export or clear

**Export logs** copies history and diagnostic logs into a dated folder on the mounted USB drive. This includes commands and controller replies for troubleshooting.

**Clear logs** deletes the saved history and logs after confirmation.

## Restore calibration

1. Select a **Probe calibration** entry. **Previous** shows the values before
   that change; **Saved** shows the values requested by it.
2. Press **Restore previous calibration** and confirm.
3. Check the restored calibration using the procedures in
   [Calibrate the probe](calibration.md).

This restores the ball diameter and X/Y/Z offsets. Work zero is unchanged.
Restoring creates a new history entry, so you can undo a restoration too.
An interrupted or failed calibration still has its previous values available.
**Clear logs** also deletes these calibration backups.
