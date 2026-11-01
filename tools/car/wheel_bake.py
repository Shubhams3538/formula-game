"""Bake source wheel shaders on their authored UVs before transforming geometry.

Invoke only from the safe Blender converter. Nested shader inputs are exposed
without executing any source text or replacing procedural textures by guesses.
"""
from pathlib import Path
import re
import hashlib

import bpy


def shaders(tree, socket, frames=()):
    """Walk only the connected surface closure, copying groups we must expose."""
    if not socket.is_linked:
        return []
    link = socket.links[0]
    node = link.from_node
    if node.type == 'GROUP':
        node.node_tree = node.node_tree.copy()
        output = next(n for n in node.node_tree.nodes if n.type == 'GROUP_OUTPUT' and n.is_active_output)
        inner = next(s for s in output.inputs if s.identifier == link.from_socket.identifier)
        return shaders(node.node_tree, inner, (*frames, (tree, node)))
    if node.type == 'GROUP_INPUT':
        parent, group = frames[-1]
        outer = next(s for s in group.inputs if s.identifier == link.from_socket.identifier)
        return shaders(parent, outer, frames[:-1])
    if node.type in {'MIX_SHADER', 'ADD_SHADER'}:
        return [item for s in node.inputs if s.type == 'SHADER' for item in shaders(tree, s, frames)]
    if node.type == 'REROUTE':
        return shaders(tree, node.inputs[0], frames)
    return [(tree, node, frames)]


def emit_channel(mat, connected, channel):
    # Replace each connected closure, preserving the source's shader mix graph.
    # Choosing one Principled node drops the wheel's additive sticker layer.
    replacements = []
    conductor = all(node.type == 'BSDF_GLOSSY' for _, node, _ in connected)
    for tree, node, _ in connected:
        emission = tree.nodes.new('ShaderNodeEmission')
        if channel == 'metallic':
            socket = node.inputs.get('Metallic')
            emission.inputs['Color'].default_value = (float(conductor),)*3 + (1,)
        else:
            socket = node.inputs.get('Base Color') or node.inputs.get('Color')
            if socket is None:
                raise RuntimeError(f'Cannot extract source colour: {mat.name}: {node.name}')
        if socket is not None:
            if socket.is_linked:
                tree.links.new(socket.links[0].from_socket, emission.inputs['Color'])
            else:
                value = socket.default_value
                emission.inputs['Color'].default_value = tuple(value) if socket.type == 'RGBA' else (value,value,value,1)
        links = [(link.from_socket, link.to_socket) for output in node.outputs for link in list(output.links)]
        for _, destination in links:
            tree.links.new(emission.outputs[0], destination)
        replacements.append((tree, emission, links))
    return replacements


def setup():
    scene = bpy.context.scene
    scene.render.engine = 'CYCLES'
    scene.cycles.samples = 4
    scene.render.threads_mode = 'FIXED'
    scene.render.threads = 8
    scene.render.bake.use_selected_to_active = False
    scene.render.bake.margin = 8
    scene.render.bake.normal_space = 'TANGENT'
    scene.cycles.device = 'CPU'
    preferences = bpy.context.preferences.addons['cycles'].preferences
    try:
        preferences.compute_device_type = 'CUDA'
        preferences.get_devices()
        devices = [device for device in preferences.devices if device.type == 'CUDA']
        if devices:
            for device in preferences.devices: device.use = device.type == 'CUDA'
            scene.cycles.device = 'GPU'
    except (TypeError, RuntimeError):
        pass  # CPU baking remains available on non-CUDA build hosts.


def bake(obj, directory, library):
    """Return one portable material per source slot, preserving per-object mapping."""
    from source_materials import material, image_node, attach_normal
    setup()
    directory = Path(directory) / 'wheel-bakes'
    directory.mkdir(parents=True, exist_ok=True)
    bpy.ops.object.select_all(action='DESELECT')
    obj.hide_set(False)
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    source_uv = obj.data.uv_layers.active
    if source_uv is None:
        raise RuntimeError(f'{obj.name}: source wheel has no UVs')
    source_uv.name = 'OriginalUV'
    source_uv.active_render = True
    source_mats = [m.copy() for m in obj.data.materials]
    # Replacing slots in place preserves polygon material indices. Clearing
    # the collection resets every polygon to slot zero in Blender.
    for slot, mat in enumerate(source_mats):
        obj.data.materials[slot] = mat
    connected = []
    for mat in source_mats:
        output = next(n for n in mat.node_tree.nodes if n.type == 'OUTPUT_MATERIAL' and n.is_active_output)
        connected.append(shaders(mat.node_tree, output.inputs['Surface']))
    size = 4096 if 'wheel_body' in obj.name else 2048 if 'grip' in obj.name or 'stickers' in obj.name else 1024
    digest = hashlib.sha256(Path(bpy.data.filepath).read_bytes())
    for script in ('wheel_bake.py', 'driver_steer.py'):
        digest.update(Path(__file__).with_name(script).read_bytes())
    directory = directory / digest.hexdigest()[:16]
    directory.mkdir(exist_ok=True)
    baked = [dict() for _ in source_mats]
    for channel in ('color', 'roughness', 'normal', 'metallic'):
        paths = [directory / (re.sub(r'[^a-zA-Z0-9_-]', '_', f'{obj.name}_{slot}_{channel}')+'.png')
                 for slot in range(len(source_mats))]
        if all(path.is_file() for path in paths):
            for slot, path in enumerate(paths):
                baked[slot][channel] = bpy.data.images.load(str(path), check_existing=False)
                baked[slot][channel].colorspace_settings.name = 'sRGB' if channel == 'color' else 'Non-Color'
                baked[slot][channel].pack()
            print('WHEEL BAKE CACHE', obj.name, channel, flush=True)
            continue
        targets = []
        for slot, mat in enumerate(source_mats):
            name = re.sub(r'[^a-zA-Z0-9_-]', '_', f'{obj.name}_{slot}_{channel}')
            image = bpy.data.images.new(name, width=size, height=size, alpha=False)
            image.colorspace_settings.name = 'sRGB' if channel == 'color' else 'Non-Color'
            image.generated_color = (.5,.5,1,1) if channel == 'normal' else (0,0,0,1)
            image.filepath_raw = str(directory / (name+'.png'))
            image.file_format = 'PNG'
            target = mat.node_tree.nodes.new('ShaderNodeTexImage')
            target.image = image
            mat.node_tree.nodes.active = target
            emission = emit_channel(mat, connected[slot], channel) if channel in ('color','metallic') else None
            targets.append((target, emission, image))
        bpy.ops.object.bake(type={'color':'EMIT','metallic':'EMIT','roughness':'ROUGHNESS','normal':'NORMAL'}[channel])
        for slot, (target, emission, image) in enumerate(targets):
            mat = source_mats[slot]
            if emission:
                for tree, node, links in emission:
                    for source, destination in links: tree.links.new(source, destination)
                    tree.nodes.remove(node)
            mat.node_tree.nodes.remove(target)
            image.save()
            image.pack()
            baked[slot][channel] = image
        print('WHEEL BAKE',obj.name,channel,size,flush=True)
    results = []
    for slot, maps in enumerate(baked):
        old = source_mats[slot].name
        name = 'source_wheel_'+old
        if old.startswith('steering_wheel_stickers.001'): name = 'wheel_stickers'
        if old.startswith('Carbon Fiber (no UV)_rough_steering_wheel.002'): name = 'wheel_legends'
        mat = material(name, (1,1,1))
        shader = mat.node_tree.nodes.get('Principled BSDF')
        for channel, socket in [('color','Base Color'),('roughness','Roughness'),('metallic','Metallic')]:
            tex = image_node(mat, maps[channel])
            mat.node_tree.links.new(tex.outputs['Color'], shader.inputs[socket])
        attach_normal(mat, maps['normal'])
        principled = next((node for _, node, _ in connected[slot] if node.type == 'BSDF_PRINCIPLED'), None)
        if principled:
            for name in ('IOR','Specular IOR Level','Coat Weight','Coat Roughness','Coat IOR'):
                if not principled.inputs[name].is_linked:
                    shader.inputs[name].default_value = principled.inputs[name].default_value
        # Multi-lobe anisotropy and transmission cannot be encoded by core glTF PBR.
        if 'Translucent' in old:
            shader.inputs['Transmission Weight'].default_value = .15
            tex = image_node(mat, maps['color'])
            mat.node_tree.links.new(tex.outputs['Color'], shader.inputs['Emission Color'])
            shader.inputs['Emission Strength'].default_value = .12
        results.append(mat)
    return results
