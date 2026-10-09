"""Builds the HUD's pictures from Overwatch's own UI art (step 12), into D:\\GenjiBridgeAssets\\hud.

Inputs are your extracted files (tools\\extract_genji.ps1 and DataTool's dump-ui-textures / extract-vector-images),
outputs stay on D:, outside the repository. Needs Pillow; the SVG icon needs resvg-py (pip install resvg-py).

  portrait_hex.png  Genji's portrait in Overwatch's hexagon: the bust inside, the helmet rising out over the top edge
  ult_disc.png      Overwatch's dark disc with its ring, behind the ultimate number
  plus.png          the health "+" icon
  glow_ring.png, dash_ring.png, flare.png, glow_dot.png   Overwatch's UI effect pictures for the HUD animations
"""
import io
import os
import sys

from PIL import Image, ImageChops, ImageFilter

RAW = r'D:\GenjiBridgeAssets\raw'
UI = r'D:\GenjiBridgeAssets\ui-textures\UITextureDump'
VEC = r'D:\GenjiBridgeAssets\ui-vectors\VectorImages'
OUT = r'D:\GenjiBridgeAssets\hud'

# Genji's bust, 180x310 with a transparent background: the picture you sent on 2026-10-08 ("use this image
# instead"), copied to D:\GenjiBridgeAssets\hud (not in your extracted files: no match among them).
PORTRAIT = os.path.join(OUT, 'portrait_source.png')
# Where its features are, in its own pixels (measured): top of the helmet, the green visor, the front of the face.
SRC_HELMET_TOP = 34
SRC_VISOR = 126
SRC_FACE_FRONT_X = 163
HEX_FILL = os.path.join(UI, '000000039E89.png')     # 100x120 filled hexagon
HEX_OUTLINE = os.path.join(UI, '000000027AB5.png')  # 120x144 hexagon outline
ULT_DISC = os.path.join(UI, '000000021660.png')     # 120x120 dark disc with ring
PLUS_SVG = os.path.join(VEC, '000000000022.svg')    # "+" icon
# Effects (found on the "blue" contact sheets, 2026-10-08): glow ring, ring of dashes, light flare, soft glow.
EFFECTS = {
    'glow_ring.png': '00000002915D.png',
    'dash_ring.png': '00000002915E.png',
    'flare.png': '00000000B666.png',
    'glow_dot.png': '000000003A2D.png',
}


def main():
    os.makedirs(OUT, exist_ok=True)
    scale = 4  # work large, Overwatch's pieces are small

    # Portrait as in your Overwatch screenshot (2026-10-08): a dark hexagon with a thick glowing cyan outline; the
    # bust inside it, cut by the outline at the bottom; the helmet rising out over the top edge, in front of the
    # outline, with a soft blue glow rising behind it. Measured on the screenshot (shares of the hexagon's height and
    # width): the helmet's top level with the hexagon's top corner, the visor 0.39 below it, the face's front at 0.88
    # of the width; the helmet stands out over the slanted top edges.
    hex_w, hex_h = 100 * scale, 120 * scale
    pad = 18 * scale                       # room for the glow
    rise = int(hex_h * 0.32)               # room above the hexagon for the helmet and the glow
    W, H = hex_w + 2 * pad, hex_h + rise + 2 * pad
    hx, hy = pad, pad + rise               # hexagon's top-left corner
    hexa = Image.open(HEX_FILL).convert('RGBA').resize((hex_w, hex_h), Image.LANCZOS).getchannel('A')
    hex_mask = Image.new('L', (W, H), 0)
    hex_mask.paste(hexa, (hx, hy))
    # Bust: scaled so helmet top and visor land where they are in Overwatch, face front at the right side.
    k = hex_h * 0.39 / (SRC_VISOR - SRC_HELMET_TOP)
    src = Image.open(PORTRAIT).convert('RGBA')
    bust = src.resize((int(src.width * k), int(src.height * k)), Image.LANCZOS)
    bx = int(hx + hex_w * 0.88 - SRC_FACE_FRONT_X * k)
    by = int(hy - hex_h * 0.02 - SRC_HELMET_TOP * k)
    layer = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    layer.alpha_composite(bust, (max(bx, 0), max(by, 0)), (max(-bx, 0), max(-by, 0)))
    # Where the bust may show: inside the outline (the hexagon shrunk by the outline's width), plus, above the
    # hexagon's middle, the hexagon's width straight up (there the helmet is drawn over the outline).
    inner = hex_mask.filter(ImageFilter.MinFilter(4 * scale + 1))
    above = Image.new('L', (W, H), 0)
    above.paste(255, (hx + 2 * scale, 0, hx + hex_w - 2 * scale, hy + hex_h // 2))
    show = ImageChops.lighter(inner, above)
    layer.putalpha(ImageChops.multiply(layer.getchannel('A'), show))
    # Outline: the hexagon's own edge, about 3 px at the final size, cyan, over a blurred glow.
    grown = hex_mask.filter(ImageFilter.MaxFilter(4 * scale + 1))
    shrunk = hex_mask.filter(ImageFilter.MinFilter(2 * scale + 1))
    ring_full = ImageChops.subtract(grown, shrunk)
    out = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    # Soft blue glow rising from the hexagon's top, behind the head.
    column = Image.new('L', (W, H), 0)
    for y in range(0, hy + hex_h // 3):
        a = int(110 * min(1.0, (y - pad * 0.3) / (hy + hex_h / 3 - pad * 0.3)) ** 1.6) if y > pad * 0.3 else 0
        column.paste(a, (hx + hex_w // 6, y, hx + hex_w * 5 // 6, y + 1))
    column = column.filter(ImageFilter.GaussianBlur(10 * scale))
    col_img = Image.new('RGBA', (W, H), (70, 170, 255, 0))
    col_img.putalpha(column)
    out.alpha_composite(col_img)
    glow = Image.new('RGBA', (W, H), (60, 220, 255, 0))
    glow.putalpha(ring_full.filter(ImageFilter.GaussianBlur(6 * scale)).point(lambda v: min(255, int(v * 1.6))))
    out.alpha_composite(glow)
    inside = Image.new('RGBA', (W, H), (14, 26, 44, 225))
    dark = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    dark.paste(inside, (0, 0), hex_mask)
    out.alpha_composite(dark)
    edge = Image.new('RGBA', (W, H), (120, 240, 255, 0))
    edge.putalpha(ring_full)
    out.alpha_composite(edge)
    out.alpha_composite(layer)
    out.resize((W // 2, H // 2), Image.LANCZOS).save(os.path.join(OUT, 'portrait_hex.png'))

    Image.open(ULT_DISC).convert('RGBA').save(os.path.join(OUT, 'ult_disc.png'))
    for name, src in EFFECTS.items():
        Image.open(os.path.join(UI, src)).convert('RGBA').save(os.path.join(OUT, name))

    try:
        sys.path.insert(0, os.environ.get('RESVG_PATH', ''))
        import resvg_py
        png = resvg_py.svg_to_bytes(svg_path=PLUS_SVG, width=96)
        Image.open(io.BytesIO(bytes(png))).convert('RGBA').save(os.path.join(OUT, 'plus.png'))
    except Exception as e:
        print('plus.png skipped:', e)

    for f in sorted(os.listdir(OUT)):
        print(f, Image.open(os.path.join(OUT, f)).size)


if __name__ == '__main__':
    main()
