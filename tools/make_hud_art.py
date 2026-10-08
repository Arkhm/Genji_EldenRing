"""Builds the HUD's pictures from Overwatch's own UI art (step 12), into D:\\GenjiBridgeAssets\\hud.

Inputs are your extracted files (tools\\extract_genji.ps1 and DataTool's dump-ui-textures / extract-vector-images),
outputs stay on D:, outside the repository. Needs Pillow; the SVG icon needs resvg-py (pip install resvg-py).

  portrait_hex.png  Genji's portrait cut to Overwatch's hexagon, with the hexagon outline on top
  ult_disc.png      Overwatch's dark disc with its ring, behind the ultimate number
  plus.png          the health "+" icon
"""
import io
import os
import sys

from PIL import Image, ImageChops

RAW = r'D:\GenjiBridgeAssets\raw'
UI = r'D:\GenjiBridgeAssets\ui-textures\UITextureDump'
VEC = r'D:\GenjiBridgeAssets\ui-vectors\VectorImages'
OUT = r'D:\GenjiBridgeAssets\hud'

PORTRAIT = os.path.join(RAW, r'Heroes\Genji\GUI\0000000040C7.01C.png')
HEX_FILL = os.path.join(UI, '000000039E89.png')     # 100x120 filled hexagon
HEX_OUTLINE = os.path.join(UI, '000000027AB5.png')  # 120x144 hexagon outline
ULT_DISC = os.path.join(UI, '000000021660.png')     # 120x120 dark disc with ring
PLUS_SVG = os.path.join(VEC, '000000000022.svg')    # "+" icon


def main():
    os.makedirs(OUT, exist_ok=True)
    scale = 4  # work large, Overwatch's pieces are small

    # Portrait cut to the hexagon, outline on top.
    outline = Image.open(HEX_OUTLINE).convert('RGBA')
    ow, oh = outline.size
    W, H = ow * scale, oh * scale
    fill = Image.open(HEX_FILL).convert('RGBA').resize((int(W * 100 / 120), int(H * 120 / 144)), Image.LANCZOS)
    mask = Image.new('L', (W, H), 0)
    mask.paste(fill.getchannel('A'), ((W - fill.width) // 2, (H - fill.height) // 2))
    portrait = Image.open(PORTRAIT).convert('RGBA')
    side = max(fill.width, fill.height)
    portrait = portrait.resize((side, side), Image.LANCZOS)
    canvas = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    canvas.paste(portrait, ((W - side) // 2, (H - side) // 2))
    canvas.putalpha(ImageChops.multiply(canvas.getchannel('A'), mask))
    ring = outline.resize((W, H), Image.LANCZOS)
    tint = Image.new('RGBA', (W, H), (110, 235, 255, 0))
    tint.putalpha(ring.getchannel('A'))
    canvas.alpha_composite(tint)
    canvas.resize((ow * 2, oh * 2), Image.LANCZOS).save(os.path.join(OUT, 'portrait_hex.png'))

    Image.open(ULT_DISC).convert('RGBA').save(os.path.join(OUT, 'ult_disc.png'))

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
