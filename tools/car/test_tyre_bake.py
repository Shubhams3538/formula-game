"""Blender checks for compound masks and linear rubber colour.

blender -b --disable-autoexec --factory-startup --python-exit-code 1 --python tools/car/test_tyre_bake.py
"""
from pathlib import Path
import sys
import tempfile

def main():
    import bpy
    import numpy as np
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import tyre_bake
    rubber = np.full((2,2,3), .04)
    mask = np.array([[0.,1.],[.5,0.]])
    for compound, colour in tyre_bake.COMPOUNDS.items():
        result = tyre_bake.composite(rubber, mask, colour)
        np.testing.assert_allclose(result[0,0], rubber[0,0])
        np.testing.assert_allclose(result[0,1], colour)
        np.testing.assert_allclose(result[1,0], (rubber[1,0] + colour) / 2)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / (compound + '.png')
            tyre_bake.save_colour('compound regression', result, path)
            image = bpy.data.images.load(str(path))
            pixels = np.array(image.pixels[:]).reshape(2,2,4)
            # 0.04 linear rubber must be ~56/255 sRGB, not 10/255.
            assert .215 < pixels[0,0,0] < .225
            np.testing.assert_allclose(pixels[0,1,:3], tyre_bake.linear_to_srgb(np.array(colour)), atol=.005)
    print('TYRE MASK AND COLOUR REGRESSION PASS')


if __name__ == '__main__': main()
