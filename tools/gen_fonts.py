"""Генератор сглаженных шрифтов для MiUX.

Создаёт:
  src/fonts/ui_mono.bin  - системный моноширинный шрифт 8x16 (DejaVu Sans Mono),
                           256 уровней прозрачности на пиксель (сглаживание).
  src/fonts/web_*.bin    - пропорциональные шрифты для браузера (DejaVu Sans)
                           нескольких размеров, обычный и жирный.

Запуск:  python3 tools/gen_fonts.py   (нужен Pillow: pip install pillow)
Шрифты DejaVu распространяются под свободной лицензией Bitstream Vera.
"""
import os, struct, sys
from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, 'src', 'fonts')
os.makedirs(OUT, exist_ok=True)

def find(names):
    dirs = ['/usr/share/fonts/truetype/dejavu', '/usr/share/fonts/TTF', '/usr/share/fonts/dejavu',
            os.path.join(ROOT, 'tools', 'ttf')]
    for d in dirs:
        for n in names:
            p = os.path.join(d, n)
            if os.path.exists(p):
                return p
    sys.exit('font not found: %s (apt install fonts-dejavu)' % names)

MONO = find(['DejaVuSansMono.ttf'])
SANS = find(['DejaVuSans.ttf'])
SANS_B = find(['DejaVuSans-Bold.ttf'])
MONO_W = find(['DejaVuSansMono.ttf'])

CHARS = list(range(0x20, 0x7F)) + list(range(0xA0, 0x100)) + list(range(0x400, 0x460)) + \
    [0x490, 0x491, 0x2013, 0x2014, 0x2018, 0x2019, 0x201C, 0x201D, 0x201E, 0x2022, 0x2026,
     0x20AC, 0x2116, 0x2122, 0x2190, 0x2191, 0x2192, 0x2193, 0x25B2, 0x25BC, 0x2713, 0x00B7]
CHARS = sorted(set(CHARS))

def ui_mono():
    W, H = 8, 16
    font = ImageFont.truetype(MONO, 13)
    asc, desc = font.getmetrics()
    base_y = 12
    data = bytearray()
    data += struct.pack('<4sHHH', b'MF16', W, H, len(CHARS))
    for cp in CHARS:
        data += struct.pack('<I', cp)
    for cp in CHARS:
        im = Image.new('L', (W * 4, H * 4), 0)
        d = ImageDraw.Draw(im)
        ch = chr(cp)
        adv = font.getlength(ch)
        ox = (W - adv) / 2.0
        d.text((ox, base_y - asc), ch, fill=255, font=font)
        cell = im.crop((0, 0, W, H))
        data += bytes(int(255 * ((v / 255.0) ** 0.75)) for v in cell.tobytes())
    open(os.path.join(OUT, 'ui_mono.bin'), 'wb').write(data)
    return data

def web_font(path, size, name):
    font = ImageFont.truetype(path, size)
    asc, desc = font.getmetrics()
    glyphs = []
    for cp in CHARS:
        ch = chr(cp)
        adv = int(round(font.getlength(ch)))
        bbox = font.getbbox(ch)
        if bbox is None or bbox[2] <= bbox[0] or bbox[3] <= bbox[1]:
            glyphs.append((cp, adv, 0, 0, 0, 0, b''))
            continue
        x0, y0, x1, y1 = bbox
        w, h = x1 - x0, y1 - y0
        im = Image.new('L', (w + 2, h + 2), 0)
        ImageDraw.Draw(im).text((-x0, -y0), ch, fill=255, font=font)
        im = im.crop((0, 0, w, h))
        glyphs.append((cp, adv, x0, y0, w, h, im.tobytes()))
    data = bytearray()
    data += struct.pack('<4sHHHH', b'MFW1', size, asc, desc, len(glyphs))
    off = 0
    table = bytearray()
    blob = bytearray()
    for cp, adv, x0, y0, w, h, bmp in glyphs:
        table += struct.pack('<IhhhHHI', cp, adv, x0, y0, w, h, len(blob))
        blob += bmp
    data += table + blob
    open(os.path.join(OUT, name), 'wb').write(data)
    return len(data)

if __name__ == '__main__':
    ui_mono()
    total = 0
    for s in (11, 13, 15, 18, 22, 28):
        total += web_font(SANS, s, 'web_%d.bin' % s)
        total += web_font(SANS_B, s, 'web_%db.bin' % s)
    total += web_font(MONO_W, 13, 'web_mono13.bin')
    print('fonts written to', OUT, 'web total', total)
