"""Prepare public release metadata. Never read private keys or task databases."""
import base64
import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path

root = Path(__file__).resolve().parents[1]
config = json.loads((root / 'release-config.json').read_text(encoding='utf-8-sig'))
assert config['repository'] == 'ikunyu/TaskDock'
assert config['appVersion'] == json.loads((root / 'src-tauri/tauri.conf.json').read_text(encoding='utf-8'))['version']
assert type(config['build']) is int and 0 < config['build'] <= 9007199254740991
release = root / 'artifacts/release'
installer = release / 'TaskDock_1.0.0_x64-setup.exe'
signature = installer.with_suffix('.exe.sig').read_text(encoding='utf-8').strip()
# Keep the wire version and asset name accepted by the original 1.0.0 updater.
# The application/installer itself uses config['appVersion'].
version = f"1.0.0+{config['build']}"
signed_text = base64.b64decode(signature, validate=True).decode('utf-8')
assert any(f'version:{version}' in line.split('\t') for line in signed_text.splitlines() if line.startswith('trusted comment: ')), 'Updater signature must bind the internal build version'
notes_path = root / 'release-notes.md'
notes = notes_path.read_text(encoding='utf-8').strip() if notes_path.exists() else 'TaskDock 更新。'
tag = f"build-{config['build']}"
asset = {'signature': signature, 'url': f'https://github.com/ikunyu/TaskDock/releases/download/{tag}/{installer.name}'}
manifest = {'version': version, 'appVersion': config['appVersion'], 'notes': notes, 'pub_date': datetime.now(timezone.utc).isoformat().replace('+00:00', 'Z'),
            'platforms': {'windows-x86_64': asset, 'windows-x86_64-nsis': asset}}
(release / 'latest.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
files = ['TaskDock.exe', installer.name, installer.name + '.sig', 'latest.json']
current_installer = f"TaskDock_{config['appVersion']}_x64-setup.exe"
if current_installer != installer.name:
    assert (release / current_installer).read_bytes() == installer.read_bytes(), 'Compatibility alias differs from versioned installer'
    files += [current_installer, current_installer + '.sig']
(release / 'UPDATE-SHA256SUMS.txt').write_text(''.join(f'{hashlib.sha256((release / name).read_bytes()).hexdigest().upper()}  {name}\n' for name in files), encoding='utf-8')
print(f'Prepared signed update {version}: {release / "latest.json"}')
