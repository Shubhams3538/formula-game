"""Review native UV0 textures on the full source mesh with named photo cameras.

Always run Blender with --factory-startup --disable-autoexec SOURCE.blend.
Uses no direct-template code or artwork. PNGs must be downscaled before review.
"""
import argparse
import math
from pathlib import Path
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
from native_paint import part_for

VIEWS = {
    'side-left': ((8, 1.8, .85), (.045, 1.8, .6), 6.1),
    'side-right': ((-8, 1.8, .85), (.045, 1.8, .6), 6.1),
    'top': ((.045, 1.8, 9), (.045, 1.8, .6), 6.1),
    'three-quarter': ((7, -7, 4.4), (.045, 1.8, .55), 6.7),
    'track-quarter': ((-7, -7, 1.8), (.045, 1.8, .55), 6.7),
    'front': ((.045, -8, 1.6), (.045, 1.8, .55), 3.5),
    'rear': ((.045, 8, 1.6), (.045, 1.8, .55), 3.5),
    'front-wing': ((.045, -3.7, 2.7), (.045, -.3, .2), 2.25),
    'nose': ((.045, -3, 4.5), (.045, .25, .3), 2.3),
    'rear-wing': ((-2.6, 6, 2), (.075, 4.5, .8), 1.9),
    'rear-wing-back': ((.075, 7, 1.15), (.075, 4.5, .85), 1.8),
    'cockpit': ((.045, 2.5, 1.1), (.045, 1.2, .65), 1.8),
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('textures', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--views', nargs='+')
    parser.add_argument('--samples', type=int, default=12)
    parser.add_argument('--gpu', action='store_true', help='use a Cycles CUDA device when available')
    args = parser.parse_args(sys.argv[sys.argv.index('--')+1:])
    args.output.mkdir(parents=True, exist_ok=True)
    deps = bpy.context.evaluated_depsgraph_get()
    carbon = bpy.data.materials.new('Neutral carbon')
    carbon.use_nodes = True
    carbon.node_tree.nodes['Principled BSDF'].inputs['Base Color'].default_value = (.006, .007, .009, 1)
    carbon.node_tree.nodes['Principled BSDF'].inputs['Roughness'].default_value = .43
    mats = {}
    objects = []
    for source in list(bpy.data.collections['fp04_tr_model'].all_objects):
        if source.type != 'MESH' or not source.data.polygons or source.hide_render:
            continue
        mesh = bpy.data.meshes.new_from_object(source.evaluated_get(deps), preserve_all_data_layers=True, depsgraph=deps)
        mesh.transform(source.matrix_world)
        obj = bpy.data.objects.new('Native review ' + source.name, mesh)
        bpy.context.scene.collection.objects.link(obj)
        part = part_for(source.name)
        if part and (args.textures / (part+'.png')).exists():
            if part not in mats:
                mat = bpy.data.materials.new('UV0 ' + part)
                mat.use_nodes = True
                shader = mat.node_tree.nodes['Principled BSDF']
                shader.inputs['Roughness'].default_value = .32
                shader.inputs['Coat Weight'].default_value = .16
                tex = mat.node_tree.nodes.new('ShaderNodeTexImage')
                tex.image = bpy.data.images.load(str((args.textures / (part+'.png')).resolve()))
                tex.image.colorspace_settings.name = 'sRGB'
                uv = mat.node_tree.nodes.new('ShaderNodeUVMap')
                uv.uv_map = mesh.uv_layers[0].name
                mat.node_tree.links.new(uv.outputs[0], tex.inputs['Vector'])
                mat.node_tree.links.new(tex.outputs['Color'], shader.inputs['Base Color'])
                mats[part] = mat
            mesh.materials.clear()
            mesh.materials.append(mats[part])
        else:
            mesh.materials.clear()
            mesh.materials.append(carbon)
        for face in mesh.polygons:
            face.material_index = 0
        objects.append(obj)
    for obj in list(bpy.data.objects):
        if obj not in objects:
            bpy.data.objects.remove(obj, do_unlink=True)
    for text in list(bpy.data.texts):
        bpy.data.texts.remove(text)
    bpy.data.orphans_purge(do_recursive=True)
    scene = bpy.context.scene
    scene.render.engine = 'CYCLES'
    scene.cycles.samples = args.samples
    scene.cycles.device = 'CPU'
    if args.gpu:
        preferences = bpy.context.preferences.addons['cycles'].preferences
        preferences.compute_device_type = 'CUDA'
        preferences.get_devices()
        devices = [device for device in preferences.devices if device.type == 'CUDA']
        if not devices:
            parser.error('no CUDA device available for native review')
        for device in preferences.devices:
            device.use = device in devices
        scene.cycles.device = 'GPU'
    scene.render.threads_mode = 'FIXED'
    scene.render.threads = 8
    scene.render.resolution_x, scene.render.resolution_y = 1280, 720
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = 'PNG'
    scene.render.image_settings.color_mode = 'RGB'
    scene.view_settings.view_transform = 'Standard'
    scene.view_settings.look = 'None'
    scene.view_settings.exposure = 0
    scene.world = bpy.data.worlds.new('Native review studio')
    scene.world.use_nodes = True
    scene.world.node_tree.nodes['Background'].inputs[0].default_value = (.3,.3,.3,1)
    scene.world.node_tree.nodes['Background'].inputs[1].default_value = .35
    for location, power, size in [((1,1,6),600,5), ((-4,2,4),350,4)]:
        light = bpy.data.lights.new('Softbox', 'AREA')
        light.energy, light.shape, light.size = power, 'DISK', size
        obj = bpy.data.objects.new('Softbox', light)
        scene.collection.objects.link(obj)
        obj.location = location
        obj.rotation_euler = (Vector((0,2,.4))-obj.location).to_track_quat('-Z','Y').to_euler()
    light = bpy.data.lights.new('Sun', 'SUN')
    light.energy = .8
    obj = bpy.data.objects.new('Sun', light)
    scene.collection.objects.link(obj)
    obj.rotation_euler = (.4,-.3,-.5)
    bpy.ops.mesh.primitive_plane_add(size=200, location=(0,0,.02))
    bpy.context.object.data.materials.append(carbon)
    camera = bpy.data.objects.new('Native camera', bpy.data.cameras.new('Native camera'))
    scene.collection.objects.link(camera)
    camera.data.type = 'ORTHO'
    scene.camera = camera
    for name, (location,target,scale) in VIEWS.items():
        if args.views and name not in args.views:
            continue
        camera.location = location
        camera.rotation_euler = (Vector(target)-camera.location).to_track_quat('-Z','Y').to_euler()
        if name == 'top':
            camera.rotation_euler.rotate_axis('Z', -math.pi/2)
        camera.data.ortho_scale = scale
        scene.render.filepath = str((args.output/(name+'.png')).resolve())
        bpy.ops.render.render(write_still=True)
        print('NATIVE_REVIEW', name, flush=True)


if __name__ == '__main__':
    main()
