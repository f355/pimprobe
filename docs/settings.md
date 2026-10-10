# Settings

Settings are saved when you accept a value and survive restarts. Distances use mm; linear feeds use mm/min.

<!-- guide-only -->
![Probe settings](images/settings.png)
<!-- /guide-only -->

## Ball diameter

**Probe ball diameter** is the effective diameter used to correct side touches. The physical ball is 2 mm, but the probe mechanism moves and the shaft bends before a touch is recorded. Calibration accounts for that movement. The machine's probe-to-spindle offsets are separate.

## Measure the effective diameter

1. Secure a gauge block or gauge pin of known size. Keep the fine feed you normally use.
2. In **Center**, measure the block with **X ridge** or **Y ridge** across its known dimension, or measure an upright pin with **Boss**. For a pin, repeat from the measured center before taking the diameter reading.
3. Read **Raw span** on the result page. **Effective diameter = |raw span − known size|**. For example, a 10 mm block with a raw span of 11.94 mm gives **1.94 mm**.
4. Enter this value in **Probe ball diameter** and measure again to check it.

For a pin, use the average of the X and Y raw spans.

## Backoff

**Retract distance** is the backoff after each coarse and fine touch. It must let the probe release. The fine search goes back to the coarse contact, allowing another 0.5 mm beyond it.

## Feeds

- **Coarse feed:** the first search for a surface.
- **Fine feed:** the second touch, used for the measurement. Lower speeds reduce probe deflection and stopping error.
- **Positioning feed:** travel between touches and backoff moves.
- **Rotary feed:** A rotation in degrees/min.

## Language

Choose English, Simplified Chinese or Swedish. Until you choose, the interface uses the CNC screen's language, or the computer's language in a local preview.

## Updates

**Check for updates** opens the release notes and installer. **Development releases** selects the rolling build instead of stable releases; that choice stays selected after installation.

**Check automatically** checks the selected channel each time the interface opens. A green dot on Settings and the update button means a new version is available. Installing it restarts the interface.

## Utilities

**Probe history** reopens saved measurements. **Export logs** copies history and diagnostic logs to a mounted USB drive. **Clear logs** deletes them after confirmation.

**Probe repeatability** measures the L bracket walls and bed repeatedly to compare readings, with optional probe retraction and homing between runs.

<!-- guide-only -->
[History and logs](history.md)

[Repeatability check](repeatability.md)

[Editing numbers](editing.md)
<!-- /guide-only -->
