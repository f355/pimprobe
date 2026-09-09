## Safe Z offset

On an Inside result, **Go to measured point** raises the probe by this distance before moving it over the measured wall or corner.

The value defaults to 40 mm and is saved when edited. Starting at G53 Z-60 with an offset of 40 gives a travel height of G53 Z-20. Choose enough lift to clear the rim and fixtures along the sideways route, within the machine's Z travel.

Inside touches and the automatic return to starting X/Y happen at starting Z. Z-only probing returns to its starting height.

The measured axes move together. An unmeasured axis stays where it is.
