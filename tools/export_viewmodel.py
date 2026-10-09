"""Step 10b (DESIGN.md 5k): exports Genji's first-person model and the animations in tools\\viewmodel_anims.py into
simple files the mod reads, in D:\\GenjiBridgeAssets\\viewmodel (outside the repository). Runs inside Steam's Blender,
in the background, with the Overwatch importer add-on loaded from D:\\GenjiBridgeAssets\\tools:

  blender.exe -b --factory-startup --python tools\\export_viewmodel.py

Output (all little-endian):
- model.bin: "GVM2", u32 vertex count, u32 index count, u32 bone count; then per vertex position (3 f32), normal
  (3 f32), texture coordinates (2 f32, as Blender has them: v up), 4 bone numbers (u8), 4 weights (f32, summing
  to 1) and the part it belongs to (u8, then 3 unused bytes); then the triangle indices (u32).
  Parts: 0 the arms; the weapons in WEAPONS below, each fixed to its hand bone the way Overwatch attaches them
  (Gameplay1P.owentity: model DA9E at hardpoint_000A, DA9D and DAA1 at hardpoint_000B; those points follow the bones
  Weapon_R and Weapon_L). Positions and normals in Blender's space at rest: metres, Z up, Genji
  looking along -Y, his eye at about (0, 0.05, 1.65).
- anims.bin: for every frame of every animation, one 3x4 matrix (12 f32, by rows) per bone that takes a vertex from
  its rest position to its posed position (skinning matrices), in Blender's space.
- anims.json: per animation its role, id, frames per second, frame count and first frame in anims.bin; anims.txt:
  the same as plain lines ("role id fps frames offset"), which the mod reads.
- albedo.png (2048x2048, from Overwatch's 4096) and emission.png (1024x1024, his green lights); each weapon's
  part<n>_albedo.png and part<n>_emission.png (1024x1024); parts.txt: "part albedo emission" per line ("-" if none).
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
MODELS = r'D:\GenjiBridgeAssets\raw\Heroes\Genji\Skin\Overwatch\Overwatch\Models'
# (part, model, attachment point, the bone it follows): the shuriken stack, the Dragonblade katana, the wakizashi
# (short blade). Found 2026-10-09 in Entities\Gameplay1P.owentity; the two sheaths on the back aren't in view.
WEAPONS = [
    (2, '00000000DA9D', 'hardpoint_000B', 'Weapon_L'),
    (3, '00000000DAA1', 'hardpoint_000B', 'Weapon_L'),
]
# The shuriken stack (part 1) moves with animations of its own (your test of 2026-10-09: fixed to the hand they sat
# closed in the palm; Overwatch fans them between the fingers and shrinks each one away as it's thrown). Those
# animations place it in the arms' own space, made as a pair with an arm animation. Exported as its 13 bones relative
# to the hand bone Weapon_R, frame by frame for each pair below, so in the mod they follow the hand in any animation
# (the resting pair for every arm animation without one).
SHURIKEN = (1, '00000000DA9E', 'Weapon_R')
SHURIKEN_PAIRS = [
    ('240E', '240D'),  # resting pose, fanned (the default): matches your clip2 at 1.0 s
]
# Tried and dropped (2026-10-09): throw 2478 with 2498, fans 243B/244F with 2451/2450, reload 2436 with 33BA (same
# lengths). In the throw one shuriken stayed hanging in the air in front of the hand, so the pairs aren't certain.
# Your clip2 shows the hand empty during throws, fans and the reload: the mod hides the stack then (arms_anim.rs).


def clip_path(anim_id):
    for dp, _, fs in os.walk(os.path.join(MODEL_DIR, 'Animations')):
        if anim_id + '.owanimclip' in fs:
            return os.path.join(dp, anim_id + '.owanimclip')
    return None


def clip_fps(path):
    with open(path, 'rb') as fh:
        head = fh.read(12)
    return struct.unpack('<f', head[8:12])[0]


def export_mesh(obj, arm, part=0, rigid_bone=None, bone_offset=0):
    """Unique (vertex, texture coordinate, normal) triangle corners with their 4 strongest bone weights (or, for a
    weapon, all on `rigid_bone`). `arm`'s bones are numbered from `bone_offset`. Also returns, per original vertex, one
    exported copy (for the check)."""
    me = obj.data
    me.calc_loop_triangles()
    uv = me.uv_layers.active.data
    normals = [n.vector for n in me.corner_normals]
    bone_index = {b.name: i + bone_offset for i, b in enumerate(arm.data.bones)}
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
                if rigid_bone is not None:
                    weights = [(1.0, rigid_bone)]
                else:
                    weights = sorted(((g.weight, group_bone[g.group]) for g in vert.groups
                                      if group_bone.get(g.group) is not None), reverse=True)[:4]
                total = sum(w for w, _ in weights) or 1.0
                weights = [(w / total, b) for w, b in weights] + [(0.0, 0)] * (4 - len(weights))
                p = world @ vert.co
                nn = (rot @ n).normalized()
                verts.append((p.x, p.y, p.z, nn.x, nn.y, nn.z, u.x, u.y, [b for _, b in weights], [w for w, _ in weights], part))
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
    body_verts = len(verts)
    images = {n.label: n.image.filepath for n in body.data.materials[0].node_tree.nodes if n.type == 'TEX_IMAGE' and n.image}
    for label, name, size in TEXTURES:
        save_texture(images[label], name, size)
        print('TEXTURE', label, images[label], '->', name)
    parts = ['0 albedo.png emission.png']

    # The weapons: each placed on its attachment point at rest, then exported as fixed to that point's bone.
    bone_index = {b.name: i for i, b in enumerate(arm.data.bones)}
    for part, model, socket, bone in WEAPONS:
        before = set(bpy.data.objects)
        bpy.ops.import_mesh.overtools2_model(directory=os.path.join(MODELS, model + '.00C') + os.sep, files=[{'name': model + '.owmdl'}])
        new = [o for o in bpy.data.objects if o not in before]
        for root in (o for o in new if o.parent is None):
            root.parent = bpy.data.objects[socket]
            root.matrix_parent_inverse.identity()
            root.matrix_basis.identity()
        bpy.context.view_layer.update()
        names = {}
        for obj in (o for o in new if o.type == 'MESH'):
            mat = obj.data.materials[0] if obj.data.materials else None
            imgs = {n.label: n.image.filepath for n in mat.node_tree.nodes if n.type == 'TEX_IMAGE' and n.image} if mat else {}
            if 'Albedo' not in imgs:
                continue  # decals
            wv, wi, _ = export_mesh(obj, arm, part, bone_index[bone])
            indices += [i + len(verts) for i in wi]
            verts += wv
            names = imgs
            print(f'WEAPON part {part} {model} on {bone}: {len(wv)} vertices, {len(wi) // 3} triangles')
        files = []
        for label in ('Albedo', 'Emission'):
            if label in names:
                files.append(f'part{part}_{label.lower()}.png')
                save_texture(names[label], files[-1], 1024)
            else:
                files.append('-')
        parts.append(f'{part} {files[0]} {files[1]}')

    # The shuriken: at the model's origin, on its own bones (numbered after the arms').
    part, model, hand_bone = SHURIKEN
    before = set(bpy.data.objects)
    bpy.ops.import_mesh.overtools2_model(directory=os.path.join(MODELS, model + '.00C') + os.sep, files=[{'name': model + '.owmdl'}])
    new = [o for o in bpy.data.objects if o not in before]
    shuriken_arm = next(o for o in new if o.type == 'ARMATURE')
    n_arm_bones = len(arm.data.bones)
    names = {}
    for obj in (o for o in new if o.type == 'MESH'):
        mat = obj.data.materials[0] if obj.data.materials else None
        imgs = {n.label: n.image.filepath for n in mat.node_tree.nodes if n.type == 'TEX_IMAGE' and n.image} if mat else {}
        if 'Albedo' not in imgs:
            continue
        wv, wi, _ = export_mesh(obj, shuriken_arm, part, None, n_arm_bones)
        indices += [i + len(verts) for i in wi]
        verts += wv
        names = imgs
        print(f'SHURIKEN part {part} {model}: {len(wv)} vertices on its {len(shuriken_arm.data.bones)} bones')
    files = []
    for label in ('Albedo', 'Emission'):
        if label in names:
            files.append(f'part{part}_{label.lower()}.png')
            save_texture(names[label], files[-1], 1024)
        else:
            files.append('-')
    parts.insert(1, f'{part} {files[0]} {files[1]}')

    with open(os.path.join(OUT, 'model.bin'), 'wb') as fh:
        # GVM3: also the number of extra bones (the shuriken's), after the arms'.
        fh.write(b'GVM3' + struct.pack('<IIII', len(verts), len(indices), n_arm_bones, len(shuriken_arm.data.bones)))
        for (px, py, pz, nx, ny, nz, u, v, bones, weights, part) in verts:
            fh.write(struct.pack('<8f4B4fB3x', px, py, pz, nx, ny, nz, u, v, *bones, *weights, part))
        fh.write(struct.pack(f'<{len(indices)}I', *indices))
    with open(os.path.join(OUT, 'parts.txt'), 'w', encoding='utf-8') as fh:
        fh.writelines(line + '\n' for line in parts)
    print(f'MODEL {len(verts)} vertices ({body_verts} the arms), {len(indices) // 3} triangles, {len(arm.data.bones)} bones')

    rest = np.array([v[:3] for v in verts[:body_verts]])
    bone_ids = np.array([v[8] for v in verts[:body_verts]])
    bone_w = np.array([v[9] for v in verts[:body_verts]])
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

    # The shuriken's bones relative to the hand, frame by frame, for each pair (shuriken.bin, shuriken.txt).
    hand = bone_index[hand_bone]
    lines = [f'hand {hand} bones {len(shuriken_arm.data.bones)}']
    with open(os.path.join(OUT, 'shuriken.bin'), 'wb') as fh:
        offset = 0
        for arm_id, shuriken_id in SHURIKEN_PAIRS:
            arm_path = clip_path(full_id(arm_id))
            sh_path = None
            for dp, _, fs in os.walk(os.path.join(MODELS, model + '.00C', 'Animations')):
                if full_id(shuriken_id) + '.owanimclip' in fs:
                    sh_path = os.path.join(dp, full_id(shuriken_id) + '.owanimclip')
            if arm_path is None or sh_path is None:
                print('MISSING shuriken pair', arm_id, shuriken_id)
                continue
            for target, path in ((arm, arm_path), (shuriken_arm, sh_path)):
                for o in bpy.data.objects:
                    o.select_set(o is target)
                bpy.context.view_layer.objects.active = target
                bpy.ops.import_mesh.overtools2_animmclip(directory=os.path.dirname(path) + os.sep, files=[{'name': os.path.basename(path)}])
            act = arm.animation_data.action
            frames = list(range(int(act.frame_range[0]), int(act.frame_range[1]) + 1))
            for fr in frames:
                scene.frame_set(fr)
                hb = arm.data.bones[hand]
                hand_skin = arm.matrix_world @ arm.pose.bones[hb.name].matrix @ hb.matrix_local.inverted() @ arm.matrix_world.inverted()
                inv = hand_skin.inverted()
                out = []
                for b in shuriken_arm.data.bones:
                    w = shuriken_arm.matrix_world @ shuriken_arm.pose.bones[b.name].matrix @ b.matrix_local.inverted() @ shuriken_arm.matrix_world.inverted()
                    rel = inv @ w
                    out.extend(rel[r][c] for r in range(3) for c in range(4))
                fh.write(struct.pack(f'<{len(out)}f', *out))
            lines.append(f'{arm_id} {shuriken_id} {len(frames)} {offset}')
            offset += len(frames)
            print('SHURIKEN PAIR', arm_id, shuriken_id, len(frames), 'frames')
    with open(os.path.join(OUT, 'shuriken.txt'), 'w', encoding='utf-8') as fh:
        fh.writelines(line + '\n' for line in lines)


main()
