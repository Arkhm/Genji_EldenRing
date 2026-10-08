"""Builds the HUD's pictures from Overwatch's own UI art (step 12), into D:\\GenjiBridgeAssets\\hud.

Inputs are your extracted files (tools\\extract_genji.ps1 and DataTool's dump-ui-textures / extract-vector-images),
outputs stay on D:, outside the repository. Needs Pillow; the SVG icon needs resvg-py (pip install resvg-py).

  portrait_hex.png  Genji's portrait cut to Overwatch's hexagon, with the hexagon outline on top
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

# The side bust Overwatch's HUD shows (helmet facing right, horn on top), 368x500.
PORTRAIT = os.path.join(RAW, r'Heroes\Genji\GUI\0000000106EC.01C.png')
# Head-and-shoulders crop of it (left, top, right, bottom), matched to your Overwatch screenshot of 2026-10-08.
PORTRAIT_CROP = (120, 36, 340, 290)
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

    # Portrait as in Overwatch's HUD: the bust fills the hexagon and the helmet rises over its top edge; dark
    # inside; a thick cyan outline with a glow.
    hex_w, hex_h = 100 * scale, 120 * scale
    pad = 18 * scale                       # room for the glow
    rise = int(hex_h * 0.22)               # how far the helmet may rise above the hexagon
    W, H = hex_w + 2 * pad, hex_h + rise + 2 * pad
    hx, hy = pad, pad + rise               # hexagon's top-left corner
    hexa = Image.open(HEX_FILL).convert('RGBA').resize((hex_w, hex_h), Image.LANCZOS).getchannel('A')
    hex_mask = Image.new('L', (W, H), 0)
    hex_mask.paste(hexa, (hx, hy))
    # Where the portrait may show: the hexagon, plus the band above its upper half (between its side corners).
    show = hex_mask.copy()
    band = Image.new('L', (int(hex_w * 0.84), rise + hex_h // 2), 255)
    show.paste(band, (hx + int(hex_w * 0.08), pad), band)
    canvas = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    inside = Image.new('RGBA', (W, H), (14, 26, 44, 225))
    canvas.paste(inside, (0, 0), hex_mask)
    bust = Image.open(PORTRAIT).convert('RGBA').crop(PORTRAIT_CROP)
    bw = int(hex_w * 1.22)
    bust = bust.resize((bw, int(bust.height * bw / bust.width)), Image.LANCZOS)
    layer = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    layer.paste(bust, (hx + (hex_w - bw) // 2, pad))
    layer.putalpha(ImageChops.multiply(layer.getchannel('A'), show))
    canvas.alpha_composite(layer)
    # Outline: Overwatch's hexagon outline, thickened, cyan, with a blurred glow under it.
    # The outline is the hexagon's own edge (Overwatch's outline texture is open at the top), about 3 px at the
    # final size.
    grown = hex_mask.filter(ImageFilter.MaxFilter(4 * scale + 1))
    shrunk = hex_mask.filter(ImageFilter.MinFilter(2 * scale + 1))
    ring_full = ImageChops.subtract(grown, shrunk)
    glow = Image.new('RGBA', (W, H), (60, 220, 255, 0))
    glow.putalpha(ring_full.filter(ImageFilter.GaussianBlur(6 * scale)).point(lambda v: min(255, int(v * 1.6))))
    under = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    under.alpha_composite(glow)
    under.alpha_composite(canvas)
    edge = Image.new('RGBA', (W, H), (120, 240, 255, 0))
    edge.putalpha(ring_full)
    under.alpha_composite(edge)
    # The helmet in front of the outline's top edge, as in Overwatch.
    top_only = Image.new('L', (W, H), 0)
    top_only.paste(band, (hx + int(hex_w * 0.08), pad), band)
    helmet = layer.copy()
    helmet.putalpha(ImageChops.multiply(helmet.getchannel('A'), top_only))
    under.alpha_composite(helmet)
    under.resize((W // 2, H // 2), Image.LANCZOS).save(os.path.join(OUT, 'portrait_hex.png'))

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
