"""Blender regression for source shader bakes and per-face material assignment.

blender -b --disable-autoexec --factory-startup --python-exit-code 1 --python tools/car/test_wheel_bake.py
"""
from pathlib import Path
import sys
import tempfile

import bpy

sys.path.insert(0, str(Path(__file__).resolve().parent))
import source_materials
import wheel_bake


def main():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    mesh = bpy.data.meshes.new('two source surfaces')
    mesh.from_pydata([(0,0,0),(1,0,0),(1,1,0),(0,1,0),
                      (2,0,0),(3,0,0),(3,1,0),(2,1,0)], [], [(0,1,2,3),(4,5,6,7)])
    obj = bpy.data.objects.new('bake-regression', mesh)
    bpy.context.scene.collection.objects.link(obj)
    uv = mesh.uv_layers.new(name='UVMap')
    for face in mesh.polygons:
        for loop, point in zip(face.loop_indices, [(0.1,.1),(.9,.1),(.9,.9),(.1,.9)]):
            uv.data[loop].uv = point
    for color, roughness, metallic in [((1,0,0),.2,0), ((0,1,0),.8,1)]:
        mesh.materials.append(source_materials.material('fixture', color, roughness, metallic))
    mesh.polygons[1].material_index = 1
    # Source translucent controls pass a shader through nested group inputs.
    mat = mesh.materials[0]
    group_tree = bpy.data.node_groups.new('shader pass-through', 'ShaderNodeTree')
    group_tree.interface.new_socket(name='Shader', in_out='INPUT', socket_type='NodeSocketShader')
    group_tree.interface.new_socket(name='Shader', in_out='OUTPUT', socket_type='NodeSocketShader')
    input_node = group_tree.nodes.new('NodeGroupInput')
    output_node = group_tree.nodes.new('NodeGroupOutput')
    group_tree.links.new(input_node.outputs[0], output_node.inputs[0])
    group = mat.node_tree.nodes.new('ShaderNodeGroup'); group.node_tree = group_tree
    mat.node_tree.links.new(mat.node_tree.nodes['Principled BSDF'].outputs[0], group.inputs[0])
    mat.node_tree.links.new(group.outputs[0], mat.node_tree.nodes['Material Output'].inputs['Surface'])
    # A saved source blend is part of the bake cache fingerprint.
    with tempfile.TemporaryDirectory() as directory:
        bpy.ops.wm.save_as_mainfile(filepath=str(Path(directory)/'source.blend'))
        mats = wheel_bake.bake(obj, directory, None)
        assert [p.material_index for p in mesh.polygons] == [0,1]
        for mat, expected in zip(mats, [(1,0,0), (0,1,0)]):
            shader = mat.node_tree.nodes.get('Principled BSDF')
            image = shader.inputs['Base Color'].links[0].from_node.image
            pixel = (image.size[0]//2 + image.size[0]*(image.size[1]//2))*4
            color = image.pixels[pixel:pixel+3]
            assert max(abs(a-b) for a,b in zip(color,expected)) < .01, (color,expected)
        print('SOURCE WHEEL BAKE REGRESSION PASS')


if __name__ == '__main__':
    main()
