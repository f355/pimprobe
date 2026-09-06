## Clearance and failed probing

Clamp the stock. If it moves under the probe, the measurement is wrong or the touch can fail.

Check the whole route for the ball and shaft, including moves between touches and the return. Machine travel limits cannot account for stock, clamps, islands, overhangs or the tool magazine.

Sideways and downward positioning moves use G38.3 at Positioning feed. An unexpected touch stops that move, backs off if possible and fails the routine. Upward Z moves and contact releases use G1. This is not a guarantee against damage. Touching the probe while idle does not stop the machine. Use the physical emergency stop if needed.

If the coarse search runs out of travel without touching, the routine fails. Check the starting position and search distance before trying again. A failed fine touch can raise a firmware alarm; the probing screen closes so you can clear it on the main screen. Inspect the machine before clearing an alarm.

Probe extension is controlled separately by the top-row switch. A routine leaves the actuator extended.
