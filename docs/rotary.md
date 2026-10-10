# Rotary

The axis-center button measures a rod held in the chuck. The other three buttons rotate a flat face until it is horizontal or vertical.

<!-- guide-only -->
![Rotary measurement buttons and distances](images/rotary.png)
<!-- /guide-only -->

## Rotary axis center

Hold a straight round rod in the chuck. Place the ball above a smooth part, roughly centered in Y. Starting Z must clear the rod along both measuring positions.

- **Rod diameter:** sets the height and sideways distance for the first side touches.
- **X distance:** signed travel to the second measuring position. Both positions must be on the rod, clear of jaws and tailstock. More separation makes small axis-angle errors easier to measure.

![Two measuring positions along X and rotated side touches](images/rotary-axis.svg)

At each X position, the probe finds the top and approximate Y center, then measures the top again at that Y. It rotates A to +90° and −90° relative to starting A while following the rod, touching the same patch of its surface from either side. Those readings give the rotation center, even if the rod is off-center in the chuck.

The probe returns to the first X position at starting Z and A. Results show both axis centers and their angles relative to X travel:

- **Set Y/Z zero:** use the first center as Y0/Z0; preserve X zero.
- **Set X/Y rotation:** align the selected WCS with the measured axis in X/Y. Requires community firmware.

## Horizontal face

Start above the Y− end of a flat face, within 20° of horizontal. Leave enough starting Z clearance for the face to rotate.

- **Y distance:** spacing from the first touch to the second in Y+.
- **Z distance:** maximum downward search from starting Z at each point.

![Horizontal face: touch at two positions separated by Y distance](images/rotary-horizontal.svg)

The probe touches both points, returns to starting Y/Z, rotates A to remove the tilt, and measures again. Both points must remain on the face after rotation.

It finishes at starting X/Y/Z with A at the corrected angle. **Set A/Z zero** uses that A angle and the average of the two surface heights.

## Vertical face

Use the Y+ or Y− picture for the direction toward the face. Start beside the **lower** touch point, with the top leaning away from the probe shaft, within 20° of vertical.

- **Y distance:** maximum search from starting Y toward the face.
- **Z distance:** spacing upward from the lower touch to the upper one.

![Vertical face: lower and upper touches, toward Y+ or Y−](images/rotary-vertical.svg)

The probe touches low, backs out to starting Y, moves up, and touches high. It returns to starting Y/Z before rotating A and checking again. Both touch points must remain on the face.

It finishes at starting X/Y/Z with A at the corrected angle. **Set A/Y zero** uses that A angle and the average of the two face positions.

<!-- guide-only -->
![Vertical face alignment results](images/rotary-vertical-result.png)

[Work coordinate systems and saved results](results.md)
<!-- /guide-only -->
