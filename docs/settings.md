# Settings

<!-- guide-only -->
![Settings and keypad](images/settings.png)
<!-- /guide-only -->

[Editing values](editing.md)

Distances are in millimeters and linear feeds are in millimeters per minute, even if the machine was in inch mode before probing.

Linear feeds are limited to the lowest X/Y/Z maximum feed in the machine settings.
Rotary feed is limited to the A-axis maximum feed. An invalid value has a red border.

## Probe ball diameter

The diameter of the probe ball. The measured surface is corrected by half this value in the touch direction. A wrong diameter gives a wrong surface coordinate.

The probe's offset from the spindle comes from the machine calibration.

## Retract distance

How far to back away along the measuring axis after each touch: once after the coarse touch and again after the fine touch.

It must be enough for the probe to release. The fine stroke searches back to the coarse contact position, with another 0.5 mm allowed beyond it.

## Positioning feed

Speed for moves between touches and for backing off after a touch. Keep it low enough for the probe and the available clearance.

Upward Z moves and releases from a measured surface use G1. Positioning along a vertical face uses G38.3 in both Z directions. Other positioning moves also use G38.3 and fail the routine if contact occurs.

## Coarse feed

Speed of the first search for each surface. It finds an approximate contact position. The probe then backs off by Retract distance.

## Fine feed

Speed of the second touch. This touch supplies the measurement. A slower feed reduces the effect of deflection and stopping delay. If the fine touch fails, the firmware can raise an alarm that must be cleared on the main screen.

## Rotary feed

Maximum chuck rotation speed in degrees per minute during rotary operations. XYZ movement uses the positioning feed.

[Clearance and failed probing](safety.md)

[Utilities](history.md)

## Software update

**Check for updates** shows the notes for the newest version in your selected channel. Turn on **Development releases** for the rolling development build, or turn it off for releases. The choice is saved across updates and restarts.

**Check automatically** checks GitHub each time the interface starts. It is on by default. A green dot on the Settings tab and Check for updates button means a new version is available in the selected channel.

**Install update** downloads and verifies the release installer, then restarts the interface to install it. Leave the machine idle with the spindle stopped while updating.
