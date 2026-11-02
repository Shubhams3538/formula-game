#!/usr/bin/env bash
# The downloaded blend is data. Its embedded Python must never execute.
set -euo pipefail
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
blender=${BLENDER:-/usr/sbin/blender}
python=${PYTHON:-python3}
source=${GEN22_SOURCE:-$repo/raw/gen22/fp04rb_new_rig.blend}
output=${1:-${ASSET_DIR:-$repo/assets}/cars/gen22}
textures=${GEN22_TEXTURES:-$repo/raw/gen22/source-textures}
work=${GEN22_WORK:-$repo/raw/gen22/pbr}
"$python" -I "$repo/tools/car/prepare_textures.py" "$textures" "$work/textures"
"$blender" -b --factory-startup --disable-autoexec "$source" --python-exit-code 1 --python "$repo/tools/car/prepare_gen22.py" -- "$output" --textures "$work/textures" --work "$work"
"$blender" -b --disable-autoexec --factory-startup --python-exit-code 1 --python "$repo/tools/car/check_gen22.py" -- "$output"
