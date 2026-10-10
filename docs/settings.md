# Settings

Settings are saved when you accept a value and survive restarts. Distances use mm; linear feeds use mm/min.

<!-- guide-only -->
![Probe settings](images/settings.png)
<!-- /guide-only -->

## Ball diameter and backoff

**Probe ball diameter** corrects the touched surface by the ball's radius. It is separate from the machine's probe-to-spindle calibration.

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
