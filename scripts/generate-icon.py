"""Generate TaskDock's code-drawn, supersampled application/NSIS artwork."""
from pathlib import Path
from PIL import Image, ImageDraw

root = Path(__file__).resolve().parents[1]
scale = 4
image = Image.new('RGBA', (256 * scale, 256 * scale), (0, 0, 0, 0))
draw = ImageDraw.Draw(image)
draw.rounded_rectangle(tuple(v * scale for v in (8, 8, 248, 248)), radius=58 * scale, fill='#6558d4')
draw.line([(x * scale, y * scale) for x, y in [(68, 131), (108, 171), (190, 87)]], fill='white', width=21 * scale, joint='curve')
draw.rounded_rectangle(tuple(v * scale for v in (87, 205, 169, 213)), radius=4 * scale, fill='#b7adf3')
directory = root / 'src-tauri' / 'icons'
directory.mkdir(parents=True, exist_ok=True)
icon = image.resize((256, 256), Image.Resampling.LANCZOS)
icon.save(directory / 'icon.ico', sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
icon.save(directory / 'icon.png')
for size, name in [(32, '32x32.png'), (128, '128x128.png'), (256, '128x128@2x.png'), (512, '512x512.png')]:
    image.resize((size, size), Image.Resampling.LANCZOS).save(directory / name)

header = Image.new('RGB', (150, 57), '#ffffff')
small = image.resize((49, 49), Image.Resampling.LANCZOS)
header.paste(small, (94, 4), small)
header.save(directory / 'installer-header.bmp')

sidebar = Image.new('RGB', (164, 314), '#f2f0fc')
mark = image.resize((112, 112), Image.Resampling.LANCZOS)
sidebar.paste(mark, (26, 50), mark)
pen = ImageDraw.Draw(sidebar)
pen.rounded_rectangle((34, 209, 130, 216), radius=3, fill='#d4cef3')
pen.rounded_rectangle((34, 232, 111, 239), radius=3, fill='#d4cef3')
pen.rounded_rectangle((34, 255, 120, 262), radius=3, fill='#d4cef3')
sidebar.save(directory / 'installer-sidebar.bmp')
