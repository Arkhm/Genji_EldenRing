"""Step 10b (DESIGN.md 5k): exports Genji's first-person model and the animations in tools\\viewmodel_anims.py into
simple files the mod reads, in D:\\GenjiBridgeAssets\\viewmodel (outside the repository). Runs inside Steam's Blender,
in the background, with the Overwatch importer add-on loaded from D:\\GenjiBridgeAssets\\tools:

  blender.exe -b --factory-startup --python tools\\export_viewmodel.py

Output (all little-endian):
- model.bin: "GVM1", u32 vertex count, u32 index count, u32 bone count; then per vertex position (3 f32), normal
  (3 f32), texture coordinates (2 f32, as Blender has them: v up), 4 bone numbers (u8) and 4 weights (f32, summing
  to 1); then the triangle indices (u32). Positions and normals in Blender's space at rest: metres, Z up, Genji
  looking along -Y, his eye at about (0, 0.05, 1.65).
- anims.bin: for every frame of every animation, one 3x4 matrix (12 f32, by rows) per bone that takes a vertex from
  its rest position to its posed position (skinning matrices), in Blender's space.
- anims.json: per animation its role, id, frames per second, frame count and first frame in anims.bin; anims.txt:
  the same as plain lines ("role id fps frames offset"), which the mod reads.
- albedo.png (2048x2048, from Overwatch's 4096) and emission.png (1024x1024, his green lights).
- export_check.txt: on two frames of every animation, the largest distance between the mesh posed the mod's way
  (these matrices and weights) and Blender's own posed mesh. Should be well under a millimetre.
"""
import json
import os
import struct
import sys

import bpy
import numpy as np

sys.path.insert(0, r'D:\GenjiBridgeAssets\tools')
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import io_scene_owm  # noqa: E402
from viewmodel_anims import ANIMS, full_id  # noqa: E402

io_scene_owm.register()

MODEL_DIR = r'D:\GenjiBridgeAssets\raw\Heroes\Genji\Skin\Overwatch\Overwatch\Models\00000000DBA6.00C'
OUT = r'D:\GenjiBridgeAssets\viewmodel'
TEXTURES = [('Albedo', 'albedo.png', 2048), ('Emission', 'emission.png', 1024)]


def clip_path(anim_id):
    for dp, _, fs in os.walk(os.path.join(MODEL_DIR, 'Animations')):
        if anim_id + '.owanimclip' in fs:
            return os.path.join(dp, anim_id + '.owanimclip')
    return None


def clip_fps(path):
    with open(path, 'rb') as fh:
        head = fh.read(12)
    return struct.unpack('<f', head[8:12])[0]


def export_mesh(obj, arm):
    """Unique (vertex, texture coordinate, normal) triangle corners with their 4 strongest bone weights. Also returns,
    per original vertex, one exported copy (for the check)."""
    me = obj.data
    me.calc_loop_triangles()
    uv = me.uv_layers.active.data
    normals = [n.vector for n in me.corner_normals]
    bone_index = {b.name: i for i, b in enumerate(arm.data.bones)}
    group_bone = {g.index: bone_index.get(g.name) for g in obj.vertex_groups}
    world = obj.matrix_world
    rot = world.to_3x3()
    corners, verts, indices, copy_of = {}, [], [], {}
    for tri in me.loop_triangles:
        for loop in tri.loops:
            v = me.loops[loop].vertex_index
            u, n = uv[loop].uv, normals[loop]
            key = (v, round(u.x, 5), round(u.y, 5), round(n.x, 3), round(n.y, 3), round(n.z, 3))
            i = corners.get(key)
            if i is None:
                i = corners[key] = len(verts)
                copy_of.setdefault(v, i)
                vert = me.vertices[v]
                weights = sorted(((g.weight, group_bone[g.group]) for g in vert.groups
                                  if group_bone.get(g.group) is not None), reverse=True)[:4]
                total = sum(w for w, _ in weights) or 1.0
                weights = [(w / total, b) for w, b in weights] + [(0.0, 0)] * (4 - len(weights))
                p = world @ vert.co
                nn = (rot @ n).normalized()
                verts.append((p.x, p.y, p.z, nn.x, nn.y, nn.z, u.x, u.y, [b for _, b in weights], [w for w, _ in weights]))
            indices.append(i)
    return verts, indices, copy_of


def skin_matrices(arm):
    """Per bone: rest position -> posed position, in world space (3x4, by rows)."""
    out = []
    aw, aw_inv = arm.matrix_world, arm.matrix_world.inverted()
    for bone in arm.data.bones:
        m = aw @ arm.pose.bones[bone.name].matrix @ bone.matrix_local.inverted() @ aw_inv
        out.extend(m[r][c] for r in range(3) for c in range(4))
    return out


def save_texture(path, name, size):
    img = bpy.data.images.load(path)
    img.scale(size, size)
    img.filepath_raw = os.path.join(OUT, name)
    img.file_format = 'PNG'
    img.save()


def main():
    os.makedirs(OUT, exist_ok=True)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_mesh.overtools2_model(directory=MODEL_DIR + os.sep, files=[{'name': '00000000DBA6.owmdl'}])
    arm = next(o for o in bpy.data.objects if o.type == 'ARMATURE')
    body = max((o for o in bpy.data.objects if o.type == 'MESH'), key=lambda o: len(o.data.vertices))  # not the decal
    scene = bpy.context.scene

    verts, indices, copy_of = export_mesh(body, arm)
    with open(os.path.join(OUT, 'model.bin'), 'wb') as fh:
        fh.write(b'GVM1' + struct.pack('<III', len(verts), len(indices), len(arm.data.bones)))
        for (px, py, pz, nx, ny, nz, u, v, bones, weights) in verts:
            fh.write(struct.pack('<8f4B4f', px, py, pz, nx, ny, nz, u, v, *bones, *weights))
        fh.write(struct.pack(f'<{len(indices)}I', *indices))
    print(f'MODEL {len(verts)} vertices, {len(indices) // 3} triangles, {len(arm.data.bones)} bones')

    images = {n.label: n.image.filepath for n in body.data.materials[0].node_tree.nodes if n.type == 'TEX_IMAGE' and n.image}
    for label, name, size in TEXTURES:
        save_texture(images[label], name, size)
        print('TEXTURE', label, images[label], '->', name)

    rest = np.array([v[:3] for v in verts])
    bone_ids = np.array([v[8] for v in verts])
    bone_w = np.array([v[9] for v in verts])
    sample = sorted(copy_of.items())[::4]
    index, check, worst = [], [], 0.0
    with open(os.path.join(OUT, 'anims.bin'), 'wb') as fh:
        offset = 0
        for role, ids in ANIMS.items():
            for short in ids:
                path = clip_path(full_id(short))
                if path is None:
                    print('MISSING', role, short)
                    continue
                bpy.context.view_layer.objects.active = arm
                bpy.ops.import_mesh.overtools2_animmclip(directory=os.path.dirname(path) + os.sep, files=[{'name': os.path.basename(path)}])
                act = arm.animation_data.action
                frames = list(range(int(act.frame_range[0]), int(act.frame_range[1]) + 1))
                for k, fr in enumerate(frames):
                    scene.frame_set(fr)
                    mats = skin_matrices(arm)
                    fh.write(struct.pack(f'<{len(mats)}f', *mats))
                    if k in (0, len(frames) // 2):
                        m = np.array(mats).reshape(-1, 3, 4)
                        posed = np.zeros_like(rest)
                        for j in range(4):
                            mj = m[bone_ids[:, j]]
                            posed += bone_w[:, j:j + 1] * (np.einsum('nij,nj->ni', mj[:, :, :3], rest) + mj[:, :, 3])
                        ev = body.evaluated_get(bpy.context.evaluated_depsgraph_get())
                        err = max(float(np.linalg.norm(posed[i] - np.array(ev.matrix_world @ ev.data.vertices[v].co)))
                                  for v, i in sample)
                        worst = max(worst, err)
                        check.append(f'{role} {short} frame {fr}: largest difference {err * 1000:.3f} mm')
                index.append({'role': role, 'id': short, 'fps': clip_fps(path), 'frames': len(frames), 'offset': offset})
                offset += len(frames)
                print('ANIM', role, short, len(frames), 'frames')
    with open(os.path.join(OUT, 'anims.json'), 'w', encoding='utf-8') as fh:
        json.dump({'bones': len(arm.data.bones), 'floats_per_bone': 12, 'anims': index}, fh, indent=1)
    # The same list as plain lines for the mod (no JSON reader there): role id fps frames offset.
    with open(os.path.join(OUT, 'anims.txt'), 'w', encoding='utf-8') as fh:
        fh.writelines(f"{a['role']} {a['id']} {a['fps']} {a['frames']} {a['offset']}\n" for a in index)
    with open(os.path.join(OUT, 'export_check.txt'), 'w', encoding='utf-8') as fh:
        fh.write(f'worst {worst * 1000:.3f} mm\n' + '\n'.join(check) + '\n')
    print(f'EXPORT DONE {len(index)} animations, worst difference {worst * 1000:.3f} mm')


main()
