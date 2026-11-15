import pathlib
import struct
import urllib.request
import zlib

from PIL import Image, ImageDraw, ImageOps

root = pathlib.Path(__file__).resolve().parents[1]
sources = root / '.research' / 'creative'
output = root / 'assets'
output.mkdir(exist_ok=True)
sources.mkdir(parents=True, exist_ok=True)
for filename, url in [
    ('legendary.jpg', 'https://i.ytimg.com/vi/aowHvqCxoj4/maxresdefault.jpg'),
    ('user-reference.jpg', 'https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcTnRbCDFXQji22EBZLAbQFauGV3_1HaG9jW_FXGWIwbxg&s=10'),
]:
    target = sources / filename
    if not target.exists():
        target.write_bytes(urllib.request.urlopen(url, timeout=30).read())

def extract(source, box, polygon):
    image = Image.open(source).convert('RGBA').crop(box)
    mask = Image.new('L', image.size)
    ImageDraw.Draw(mask).polygon(polygon, fill=255)
    image.putalpha(mask)
    image.thumbnail((28, 30), Image.Resampling.NEAREST)
    sprite = Image.new('RGBA', (32, 32))
    sprite.alpha_composite(image, ((32-image.width)//2, (32-image.height)//2))
    return sprite

legendary = extract(sources / 'legendary.jpg', (218,197,326,321),
    [(7,47),(14,31),(26,20),(31,8),(43,4),(76,4),(76,10),(88,10),(88,24),(99,24),(101,41),(105,48),(105,81),(100,88),(99,108),(84,108),(84,117),(36,119),(26,113),(17,112),(12,101),(7,93),(3,70)])
mythical = extract(sources / 'user-reference.jpg', (257,158,356,269),
    [(8,43),(16,28),(26,23),(26,15),(39,15),(39,8),(52,8),(52,6),(62,11),(68,12),(68,19),(79,19),(79,29),(90,30),(93,40),(93,59),(85,62),(86,79),(91,80),(90,96),(80,96),(80,106),(41,106),(35,102),(22,101),(14,90),(9,79),(7,63)])
for name, sprite in [('legendary-infinity', legendary), ('mythical-infinity', mythical)]:
    sprite.save(output / f'{name}.png')

url = 'https://cdn.growtopiawiki.com/sprites/7188.png?version=c632cf11f262b783ea535b0ecaeb671c'
source = sources / 'blue-gem-lock.png'
if not source.exists():
    request = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0', 'Referer': 'https://growtopiawiki.com/'})
    source.write_bytes(urllib.request.urlopen(request, timeout=30).read())
blue = Image.open(source).convert('RGBA')
white = ImageOps.grayscale(blue).convert('RGBA')
white.putalpha(blue.getchannel('A'))
white = white.resize((32,32),Image.Resampling.NEAREST)
white.save(output / 'white-gem-lock.png')

sheet = Image.new('RGBA',(128,128))
for index,sprite in enumerate([legendary,mythical,white]):
    sheet.alpha_composite(sprite,(index*32,0))
sheet.save(output / 'custom-icons.png')
pixels = sheet.transpose(Image.Transpose.FLIP_TOP_BOTTOM).tobytes()
header = bytearray(124)
header[:6] = b'RTTXTR'
struct.pack_into('<5I',header,8,128,128,5121,128,128)
header[28] = 1
struct.pack_into('<H',header,30,1)
struct.pack_into('<I',header,32,1)
struct.pack_into('<3I',header,100,128,128,len(pixels))
raw = bytes(header)+pixels
compressed = zlib.compress(raw)
pack = bytearray(32)
pack[:6] = b'RTPACK'
pack[6:8] = b'\x01\x01'
struct.pack_into('<II',pack,8,len(compressed),len(raw))
pack[16] = 1
(output / 'custom-icons.rttex').write_bytes(pack+compressed)
preview = Image.new('RGBA',(384,128),(45,45,50,255))
for index,sprite in enumerate([legendary,mythical,white]):
    preview.alpha_composite(sprite.resize((128,128),Image.Resampling.NEAREST),(index*128,0))
preview.save(output / 'preview.png')
print('Saved reference-derived sprites, preview and RTTEX atlas to',output)
