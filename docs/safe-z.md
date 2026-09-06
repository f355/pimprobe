## Safe Z offset

How far to raise after the last X/Y touch and backoff, before moving over the measured point or center. It applies to Inside probing and to Hole, Pocket and valleys on Center.

The value is shared between Inside and Center and defaults to 40 mm. Starting at G53 Z-60 with an offset of 40 gives a final height of G53 Z-20. Choose enough lift to clear the rim and fixtures along the sideways route, within the machine's Z travel.

Touches and moves between internal walls happen at starting Z. Z-only probing returns to its starting height.
