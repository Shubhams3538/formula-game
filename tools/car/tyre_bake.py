"""Bake source tyre UV0 once, then colour only its Pirelli alpha mask.

Run inside Blender with --disable-autoexec. Runtime uses ordinary opaque PBR
textures so compound selection adds neither geometry nor a shader variant.
"""
import hashlib
from pathlib import Path

import bpy
import numpy as np

COMPOUNDS = {'soft': (.8, .012, .018), 'medium': (1., .7, .005), 'hard': (.85, .85, .85)}


def composite(rubber, mask, colour):
    return rubber * (1 - mask[..., None]) + np.asarray(colour) * mask[..., None]


def linear_to_srgb(rgb):
    return np.where(rgb <= .0031308, rgb * 12.92, 1.055 * rgb ** (1 / 2.4) - .055)


def save_colour(name, rgb, path):
    from source_materials import generated_image
    image = generated_image(name, np.concatenate((linear_to_srgb(rgb), np.ones((*rgb.shape[:2],1))), axis=-1))
    image.filepath_raw = str(path.resolve())
    image.file_format = 'PNG'
    image.save()
    bpy.data.images.remove(image)


def bake(obj, work, output):
    from source_materials import generated_image, image_node, material
    from wheel_bake import setup

    digest = hashlib.sha256(Path(bpy.data.filepath).read_bytes())
    digest.update(Path(__file__).read_bytes())
    directory = Path(work) / 'tyre-bakes' / digest.hexdigest()[:16]
    directory.mkdir(parents=True, exist_ok=True)
    paths = {channel: directory / (channel + '.png') for channel in ('rubber', 'mask')}
    if not all(path.is_file() for path in paths.values()):
        setup()
        # This small emission bake avoids competing with the live Vulkan renderer.
        bpy.context.scene.cycles.device = 'CPU'
        bpy.context.scene.cycles.samples = 1
        bpy.ops.object.select_all(action='DESELECT')
        obj.hide_set(False)
        obj.select_set(True)
        bpy.context.view_layer.objects.active = obj
        original = obj.data.materials[0]
        source = original.copy()
        obj.data.materials[0] = source
        nodes, links = source.node_tree.nodes, source.node_tree.links
        output_node = next(n for n in nodes if n.type == 'OUTPUT_MATERIAL' and n.is_active_output)
        emission = nodes.new('ShaderNodeEmission')
        links.new(emission.outputs[0], output_node.inputs['Surface'])
        # Keep both source dirt images and their original coordinate nodes.
        value = nodes.new('ShaderNodeMath')
        value.operation = 'MULTIPLY_ADD'
        value.inputs[1].default_value = .01
        value.inputs[2].default_value = .035
        links.new(nodes['Mix'].outputs['Result'], value.inputs[0])
        for channel, socket in [('rubber', value.outputs[0]),
                                ('mask', nodes['Image Texture.003'].outputs['Alpha'])]:
            image = bpy.data.images.new('tyre_' + channel, width=2048, height=2048, alpha=False)
            image.colorspace_settings.name = 'Non-Color' if channel == 'mask' else 'sRGB'
            image.generated_color = (0,0,0,1) if channel == 'mask' else (.04,.04,.04,1)
            target = nodes.new('ShaderNodeTexImage')
            target.image = image
            nodes.active = target
            links.new(socket, emission.inputs['Color'])
            bpy.ops.object.bake(type='EMIT')
            image.filepath_raw = str(paths[channel].resolve())
            image.file_format = 'PNG'
            image.save()
            nodes.remove(target)
        obj.data.materials[0] = original
        bpy.data.materials.remove(source)
    rubber_image = bpy.data.images.load(str(paths['rubber'].resolve()), check_existing=False)
    mask_image = bpy.data.images.load(str(paths['mask'].resolve()), check_existing=False)
    mask_image.colorspace_settings.name = 'Non-Color'
    rubber = np.empty(2048*2048*4, dtype=np.float32)
    mask = np.empty_like(rubber)
    rubber_image.pixels.foreach_get(rubber)
    mask_image.pixels.foreach_get(mask)
    rubber = rubber.reshape(2048,2048,4)[...,:3]
    # Blender byte-image pixels are encoded sRGB, while our albedo math is linear.
    rubber = np.where(rubber <= .04045, rubber / 12.92, ((rubber + .055) / 1.055) ** 2.4)
    covered = rubber[...,0] > 0
    assert covered.any() and .03 < np.median(rubber[covered]) < .05, 'invalid linear rubber bake'
    # Baking clears unused UV space to black; extend neutral rubber into it.
    rubber = np.clip(rubber, .035, .045)
    mask = mask.reshape(2048,2048,4)[...,0]
    assert mask.max() > .95 and .001 < (mask > .5).mean() < .3, 'missing Pirelli alpha art'
    output = Path(output) / 'tyres'
    output.mkdir(parents=True, exist_ok=True)
    # Ship the neutral rubber and greyscale mask as well as ready-to-load variants.
    import shutil
    save_colour('tyre_rubber', rubber, output / 'rubber.png')
    shutil.copyfile(paths['mask'], output / 'marking-mask.png')
    medium = None
    for compound, colour in COMPOUNDS.items():
        rgb = composite(rubber, mask, colour)
        encoded = linear_to_srgb(rgb)
        image = generated_image('tyre_' + compound, np.concatenate((encoded, np.ones((2048,2048,1))), axis=-1))
        for lod, size in [('',2048), ('lod1',2048), ('lod2',1024)]:
            destination = output / lod
            destination.mkdir(exist_ok=True)
            if image.size[0] != size: image.scale(size,size)
            image.filepath_raw = str((destination / (compound + '.png')).resolve())
            image.file_format = 'PNG'
            image.save()
        if compound == 'medium':
            medium = bpy.data.images.load(str((output / 'medium.png').resolve()), check_existing=False)
            medium.pack()
        bpy.data.images.remove(image)
    mat = material('tyre', (1,1,1), .7)
    shader = mat.node_tree.nodes.get('Principled BSDF')
    shader.inputs['Specular IOR Level'].default_value = .5
    texture = image_node(mat, medium)
    mat.node_tree.links.new(texture.outputs['Color'], shader.inputs['Base Color'])
    print('TYRE BAKE', directory, 'mask coverage', float((mask > .5).mean()), flush=True)
    return mat
