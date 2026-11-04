#!/usr/bin/env python3
"""Rebuild embedded CC0 Poly Haven cubemaps. Requires numpy, Pillow and opencv-python."""
import argparse
import hashlib
from pathlib import Path
import subprocess
import cv2
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
SOURCES = {
    'clear': ('qwantani_noon_puresky', 'd070d48c8e802c92027b0f7b1965b5bd820b0d25afca225503abfec67e00ad07'),
    'storm': ('overcast_soil_puresky', '5414e1ac0c35e451d28595b2a2cba05e2ca301fdeb755b0a469408443093dafc'),
}


def build(source, destination):
    panorama = cv2.imread(str(source), cv2.IMREAD_UNCHANGED)[..., ::-1]
    height, width, _ = panorama.shape
    # Preserve cloud contrast while fitting radiance into an embedded sRGB image.
    scale = np.percentile(panorama[:height // 2], 95)
    panorama = np.clip(panorama / scale, 0., 1.)
    for size in (128, 256, 512):
        u, v = np.meshgrid((np.arange(size) + .5) / size * 2 - 1,
                           (np.arange(size) + .5) / size * 2 - 1)
        one = np.ones_like(u)
        faces = [(one, -v, -u), (-one, -v, u), (u, one, v),
                 (u, -one, -v), (u, -v, one), (-u, -v, -one)]
        images = []
        for x, y, z in faces:
            longitude = np.arctan2(x, z)
            latitude = np.arctan2(y, np.sqrt(x * x + z * z))
            px = ((longitude / (2 * np.pi) + .5) * width - .5).astype(np.float32)
            py = ((.5 - latitude / np.pi) * height - .5).astype(np.float32)
            face = cv2.remap(panorama, px, py, cv2.INTER_LINEAR, borderMode=cv2.BORDER_WRAP)
            face = np.where(face <= .0031308, face * 12.92, 1.055 * face ** (1 / 2.4) - .055)
            images.append(np.round(face * 255).astype(np.uint8))
        Image.fromarray(np.concatenate(images)).save(str(destination) + f'-{size}.png', optimize=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cache', type=Path, default=ROOT / 'target/sky-sources')
    args = parser.parse_args()
    args.cache.mkdir(parents=True, exist_ok=True)
    output = ROOT / 'game/sky'
    output.mkdir(exist_ok=True)
    for name, (asset, digest) in SOURCES.items():
        source = args.cache / f'{name}.hdr'
        if not source.exists():
            partial = source.with_suffix(".hdr.part")
            subprocess.run(['curl', '-fL', '--retry', '2', '--max-time', '120',
                            f'https://dl.polyhaven.org/file/ph-assets/HDRIs/hdr/2k/{asset}_2k.hdr',
                            '-o', str(partial)], check=True)
            if digest and hashlib.sha256(partial.read_bytes()).hexdigest() != digest:
                partial.unlink()
                raise RuntimeError(f'Source checksum mismatch: {partial}')
            partial.replace(source)
        if digest and hashlib.sha256(source.read_bytes()).hexdigest() != digest:
            raise RuntimeError(f'Source checksum mismatch: {source}')
        build(source, output / name)


if __name__ == '__main__':
    main()
