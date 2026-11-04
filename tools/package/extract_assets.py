"""Extract the asset-only release ZIP without allowing checkout files to be replaced."""

import argparse
import hashlib
import io
from pathlib import Path, PurePosixPath
import shutil
import stat
import zipfile


def extract(archive: Path, destination: Path, sha256: str) -> None:
    content = archive.read_bytes()
    if hashlib.sha256(content).hexdigest() != sha256:
        raise ValueError("Asset archive SHA-256 does not match the pinned release assets")
    if destination.exists():
        raise ValueError(f"Asset destination already exists: {destination}")
    with zipfile.ZipFile(io.BytesIO(content)) as source:
        members = source.infolist()
        for member in members:
            path = PurePosixPath(member.filename)
            if (
                not path.parts
                or path.parts[0] != "assets"
                or any(part in (".", "..") or ":" in part for part in path.parts)
                or "\\" in member.filename
                or path.is_absolute()
                or stat.S_ISLNK(member.external_attr >> 16)
            ):
                raise ValueError(f"Not an asset path: {member.filename}")
        destination.mkdir(parents=True)
        try:
            for member in members:
                relative = PurePosixPath(member.filename).parts[1:]
                if not relative or member.is_dir():
                    continue
                target = destination.joinpath(*relative)
                target.parent.mkdir(parents=True, exist_ok=True)
                with source.open(member) as data, target.open("xb") as output:
                    shutil.copyfileobj(data, output)
        except BaseException:
            shutil.rmtree(destination)
            raise


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--sha256", required=True)
    args = parser.parse_args()
    extract(args.archive, args.destination, args.sha256)
