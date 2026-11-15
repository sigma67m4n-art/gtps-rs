# Custom icons

The Infinity fist icons are extracted from the supplied Creative PS image
references, with hand-traced transparency masks and nearest-neighbor reduction.
They preserve the reference colors and markings, including compression blur.
They are reconstructions, not verified copies of the original Creative PS sprites.

- Legendary: https://i.ytimg.com/vi/aowHvqCxoj4/maxresdefault.jpg
- Mythical: https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcTnRbCDFXQji22EBZLAbQFauGV3_1HaG9jW_FXGWIwbxg&s=10
- White Gem Lock: grayscale recolor of https://cdn.growtopiawiki.com/sprites/7188.png

`custom-icons.rttex` is a 128×128 RGBA RTTEX atlas. Cells (0,0), (1,0), (2,0)
contain Legendary, Mythical and White Gem Lock. PNGs and a scaled preview are
included. `scripts/build_icons.py` rebuilds them using Pillow and the reference
images in `.research/creative`. RTTEX layout reference: StileDevs/RTPackJS.
