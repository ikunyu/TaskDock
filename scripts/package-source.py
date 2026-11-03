"""Archive source and docs only; never include runtime data or tool caches."""
from pathlib import Path
import json
import re
import zipfile

root = Path(__file__).resolve().parents[1]
version = json.loads((root / 'package.json').read_text(encoding='utf-8'))['version']
if not re.fullmatch(r'\d+\.\d+\.\d+', version):
    raise ValueError('Expected a release version')
output = root / 'artifacts' / 'release'
output.mkdir(parents=True, exist_ok=True)
archive = output / f'TaskDock-{version}-source.zip'
folders = ['src', 'src-tauri', 'scripts', 'tests', 'docs', '.github']
root_files = [
    'release-config.json', 'release-notes.md', 'README.md', 'CONTRIBUTING.md', 'package.json', 'package-lock.json', 'index.html', 'widget-lock.html',
    'tsconfig.json', 'vite.config.ts', '.editorconfig', '.gitattributes',
    '.gitignore', '.prettierignore', '.prettierrc.json',
]
files = [root / name for name in root_files]
for folder in folders:
    for path in (root / folder).rglob('*'):
        relative = path.relative_to(root)
        if {'target', 'gen', '__pycache__', 'node_modules', '.tools', '.private'} & set(relative.parts):
            continue
        if relative.parts[:2] in {('src-tauri', 'data'), ('src-tauri', 'logs'), ('src-tauri', 'webview')} or any(part.startswith('.taskdock-import-') for part in relative.parts):
            continue
        if path.is_symlink() or not path.is_file() or not path.resolve().is_relative_to(root):
            continue
        if any(part.startswith('.env') for part in relative.parts):
            continue
        if path.suffix.lower() in {'.key', '.pem', '.p12', '.pfx'}:
            continue
        if re.search(r'\.(?:db|sqlite3?)(?:-wal|-shm)?$|\.log$', path.name, re.I):
            continue
        files.append(path)
with zipfile.ZipFile(archive, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=9) as bundle:
    for path in sorted(set(files)):
        bundle.write(path, Path(f'TaskDock-{version}') / path.relative_to(root))
print(f'{archive} ({len(set(files))} source files)')
