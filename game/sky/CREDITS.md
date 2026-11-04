# Skybox sources

Embedded cubemaps derived from Poly Haven HDRIs, downloaded 2026-11-03.
All original images are CC0 1.0 Universal, public domain dedication.
Commercial use and redistribution are permitted. https://polyhaven.com/license
Legal text: https://creativecommons.org/publicdomain/zero/1.0/legalcode

- Clear: Qwantani Noon (Pure Sky), Greg Zaal and Jarod Guest.
  https://polyhaven.com/a/qwantani_noon_puresky
- Storm: Overcast Soil (Pure Sky), Jarod Guest and Sergej Majboroda.
  https://polyhaven.com/a/overcast_soil_puresky

Rebuild: `python3 tools/sky/build_skyboxes.py`. Source URLs and SHA-256 hashes
are pinned in that script. It needs numpy, Pillow and opencv-python. The
2K panoramas are converted to 128/256/512-pixel cubemap faces, radiance-scaled
and encoded as sRGB PNGs. Bevy embeds these files into the executable, so
packaged games need no separate download or updated external asset bundle.
Storm uses the same dry track and tyres as Clear. The thunder is synthesized
in the game's existing audio mixer and has no third-party audio source.
