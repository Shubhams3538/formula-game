"""Artwork preparation must keep negative space and the source mark's proportions."""
from pathlib import Path
import tempfile
import unittest

import numpy as np
from PIL import Image
from prepare_native_logos import prepare, transparent_background


class NativeLogoTest(unittest.TestCase):
    def test_source_crop_excludes_campaign_marks_before_extracting_ink(self):
        pixels = np.full((50, 100, 4), 255, np.uint8)
        pixels[5:15, 5:15, :3] = (220, 20, 20)
        pixels[25:35, 30:90, :3] = (220, 20, 20)
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / 'brand.png'
            Image.fromarray(pixels).save(source)
            result = prepare({'source_crop': [20, 20, 95, 40], 'red_ink': True}, source)
            self.assertEqual(result.size, (60, 10))
            self.assertEqual(result.getpixel((1, 1)), (255, 255, 255, 255))

    def test_only_exterior_white_is_removed(self):
        pixels = np.full((20, 30, 4), 255, np.uint8)
        pixels[3:17, 5:25, :3] = 0
        pixels[7:13, 10:20, :3] = 255
        result = np.array(transparent_background(Image.fromarray(pixels)))
        self.assertEqual(result[0,0,3], 0)
        self.assertEqual(result[9,15,3], 255)
        self.assertEqual(result[4,7,3], 255)

    def test_opaque_download_canvas_cannot_become_a_white_decal_rectangle(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / 'brand.png'
            pixels = np.full((60,60,4),255,np.uint8)
            pixels[20:40,10:50,:3] = (30,30,30)
            Image.fromarray(pixels).save(source)
            result = prepare({'remove_all_white': True, 'color': 'white'}, source)
            self.assertEqual(result.size, (40,20))
            self.assertEqual(result.getpixel((1,1)), (255,255,255,255))

    def test_pirelli_brand_tile_becomes_only_transparent_wordmark(self):
        pixels = np.full((20, 60, 4), (255,220,0,255), np.uint8)
        pixels[5:15, 10:50] = (235,20,25,255)
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / 'tile.png'
            Image.fromarray(pixels).save(source)
            result = prepare({'red_ink': True}, source)
            self.assertEqual(result.size, (40,10))
            self.assertEqual(result.getpixel((20,5)), (255,255,255,255))


if __name__ == '__main__':
    unittest.main()
