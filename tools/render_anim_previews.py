"""Renders Genji's animations into small preview frames, to see which move each animation (and so each sound) is.
Runs inside Blender (Steam's Blender 5.2), in the background, with the Overwatch importer add-on io_scene_owm loaded
from D:\\GenjiBridgeAssets\\tools (nothing is installed into Blender):

  blender.exe -b --factory-startup --python tools\\render_anim_previews.py

Output: D:\\GenjiBridgeAssets\\anim_previews\\<1P|3P>_<animation id>\\0000.png ... (outside the repository);
tools\\make_sound_picker.py turns them into looping GIFs and shows them in the picker.

Only animations that play sounds (an AnimationEffects folder with Sounds) are rendered, unless `-- --all-1p` is given:
then every first-person animation (129, for the animation picker of step 10a; DESIGN.md 5k), and no third person,
seen exactly from Genji's eye with your Overwatch field of view (103 degrees at 16:9) on your 2580x1080 screen, into
D:\\GenjiBridgeAssets\\anim_previews_eye (2026-10-09: the framed previews were hard to tell apart; from the eye they
look as in Overwatch):

  blender.exe -b --factory-startup --python tools\\render_anim_previews.py -- --all-1p

First person: Genji's first-person model (00000000DBA6), seen from just behind and above his eye (his eye is about (0,
0.05, 1.65) in Blender's space, looking along -Y, found by trying positions on 2026-10-09; seen from there the arms are
mostly cut off by the screen's edge, so the previews step back). Third person: his model (00000000DB92) from the front.
Both framed to fit each move.
"""
import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, r'D:\GenjiBridgeAssets\tools')
SKIN = r'D:\GenjiBridgeAssets\raw\Heroes\Genji\Skin\Overwatch\Overwatch'
OUT = r'D:\GenjiBridgeAssets\anim_previews'
MODELS = [('1P', '00000000DBA6'), ('3P', '00000000DB92')]
STEP = 2  # render every 2nd frame (15 pictures a second for 30 fps animations)
SIZE = (320, 180)

ALL_1P = '--all-1p' in sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else False
EYE = Vector((0.0, 0.05, 1.65))
EYE_SIZE = (430, 180)  # your screen's shape (2580x1080)
if ALL_1P:
    MODELS = MODELS[:1]
    OUT = r'D:\GenjiBridgeAssets\anim_previews_eye'
    SIZE = EYE_SIZE

import io_scene_owm  # noqa: E402

io_scene_owm.register()


def sound_animations(model_dir):
    root = os.path.join(model_dir, 'AnimationEffects')
    if not os.path.isdir(root):
        return []
    return sorted(a for a in os.listdir(root) if os.path.isdir(os.path.join(root, a, 'Sounds')))


def all_animations(model_dir):
    found = set()
    for _, _, fs in os.walk(os.path.join(model_dir, 'Animations')):
        found.update(f[:-len('.owanimclip')] for f in fs if f.endswith('.owanimclip'))
    return sorted(found)


def clip_path(model_dir, anim_id):
    for dp, _, fs in os.walk(os.path.join(model_dir, 'Animations')):
        if anim_id + '.owanimclip' in fs:
            return os.path.join(dp, anim_id + '.owanimclip')
    return None


def setup_scene():
    scene = bpy.context.scene
    scene.render.engine = 'BLENDER_WORKBENCH'
    scene.display.shading.color_type = 'TEXTURE'
    scene.display.shading.light = 'STUDIO'
    scene.render.resolution_x, scene.render.resolution_y = SIZE
    scene.render.image_settings.file_format = 'PNG'
    world = bpy.data.worlds.new('w')
    scene.world = world
    world.color = (0.08, 0.09, 0.12)
    return scene


def camera(name):
    data = bpy.data.cameras.new(name)
    data.sensor_fit = 'HORIZONTAL'
    data.clip_start = 0.02
    cam = bpy.data.objects.new(name, data)
    bpy.context.collection.objects.link(cam)
    return cam


def bounds(scene, meshes, frames):
    lo, hi = Vector((1e9,) * 3), Vector((-1e9,) * 3)
    for fr in frames:
        scene.frame_set(fr)
        dg = bpy.context.evaluated_depsgraph_get()
        for m in meshes:
            ev = m.evaluated_get(dg)
            verts = ev.data.vertices
            for i in range(0, len(verts), 7):
                w = ev.matrix_world @ verts[i].co
                lo = Vector(map(min, lo, w))
                hi = Vector(map(max, hi, w))
    return lo, hi


def render_model(label, model_id):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = setup_scene()
    model_dir = os.path.join(SKIN, 'Models', model_id + '.00C')
    bpy.ops.import_mesh.overtools2_model(directory=model_dir + os.sep, files=[{'name': model_id + '.owmdl'}])
    arm = next(o for o in bpy.data.objects if o.type == 'ARMATURE')
    meshes = [o for o in bpy.data.objects if o.type == 'MESH']
    cam = camera(label)
    scene.camera = cam
    for anim_id in (all_animations(model_dir) if ALL_1P else sound_animations(model_dir)):
        clip = clip_path(model_dir, anim_id)
        out_dir = os.path.join(OUT, f'{label}_{anim_id}')
        if clip is None or os.path.isfile(os.path.join(out_dir, 'done')):
            continue
        os.makedirs(out_dir, exist_ok=True)
        bpy.context.view_layer.objects.active = arm
        arm.select_set(True)
        bpy.ops.import_mesh.overtools2_animmclip(directory=os.path.dirname(clip) + os.sep, files=[{'name': os.path.basename(clip)}])
        act = arm.animation_data.action if arm.animation_data else None
        if act is None:
            print('NO ACTION', label, anim_id)
            continue
        first, last = int(act.frame_range[0]), int(act.frame_range[1])
        frames = list(range(first, last + 1, STEP))
        if ALL_1P:
            # Genji's eye, looking along -Y; Overwatch's 103 degrees at 16:9, widened to the screen's shape.
            vertical = 2 * math.atan(math.tan(math.radians(103.0) / 2) * 9 / 16)
            cam.data.angle = 2 * math.atan(math.tan(vertical / 2) * SIZE[0] / SIZE[1])
            cam.data.clip_start = 0.01
            cam.location = EYE
            cam.rotation_euler = Vector((0, -1, 0)).to_track_quat('-Z', 'Y').to_euler()
        else:
            # Framed to fit the whole move: first person from just behind and above the eye (+Y is behind), third
            # person from the front.
            lo, hi = bounds(scene, meshes, frames[::3] or frames)
            center = (lo + hi) / 2
            extent = max((hi - lo).length, 0.5)
            cam.data.angle = math.radians(45.0)
            if label == '1P':
                direction = Vector((0.15, 1.0, 0.55)).normalized()
            else:
                direction = Vector((0.45, -1.0, 0.25)).normalized()  # from the front (-Y), a little right and above
            cam.location = center + direction * (extent / (2 * math.tan(cam.data.angle / 2)) * 1.1)
            cam.rotation_euler = (center - cam.location).to_track_quat('-Z', 'Y').to_euler()
        for i, fr in enumerate(frames):
            scene.frame_set(fr)
            scene.render.filepath = os.path.join(out_dir, f'{i:04d}.png')
            bpy.ops.render.render(write_still=True)
        with open(os.path.join(out_dir, 'done'), 'w') as fh:
            fh.write(f'{len(frames)} frames, {act.frame_range[1] - act.frame_range[0]:.0f} animation frames\n')
        print('RENDERED', label, anim_id, len(frames))


for label, model_id in MODELS:
    render_model(label, model_id)
print('ALL DONE')
