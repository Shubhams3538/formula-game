"""Losslessly re-encode the source RGB pixels and AO red channel.

Invoke with python3 -I. Downloads remain untrusted data, never imported code.
The original copies stay intact; outputs are private build intermediates.
"""
import argparse
from pathlib import Path

from PIL import Image


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('source',type=Path)
    parser.add_argument('output',type=Path)
    args=parser.parse_args()
    for folder in ('textures_audi','shadow_maps','material_maps'):
        destination=args.output/folder
        destination.mkdir(parents=True,exist_ok=True)
        for path in sorted((args.source/folder).glob('*.png')):
            with Image.open(path) as image:
                image.load()
                # glTF AO reads only red; body shaders use opaque RGB colour.
                image=image.convert('RGB')
                if folder=='shadow_maps': image=image.getchannel('R')
                image.save(destination/path.name,format='PNG',compress_level=9)
            print('TEXTURE',folder,path.name,flush=True)


if __name__=='__main__': main()
