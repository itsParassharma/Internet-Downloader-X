"""Keep the unpacked extension identity and Rust native-host allowlist in sync.

No store listing is created. After a store draft exists, --public-key accepts
its PUBLIC key PEM; rebuild the app and extension together afterwards.
"""
import argparse
import base64
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--public-key', type=Path)
parser.add_argument('--check', action='store_true')
args = parser.parse_args()
manifest_path = root / 'browser-extension/chromium/manifest.json'
manifest = json.loads(manifest_path.read_text(encoding='utf-8'))
if args.public_key:
    assert not args.check, 'Choose --public-key or --check'
    text = args.public_key.read_text(encoding='ascii')
    assert 'PRIVATE' not in text, 'Only a public key is accepted'
    text = ''.join(line.strip() for line in text.splitlines() if not line.startswith('-----'))
    key = base64.b64decode(text, validate=True)
    assert len(key) >= 128 and key[0] == 0x30, 'Expected DER public key'
    manifest['key'] = base64.b64encode(key).decode('ascii')
    manifest_path.write_text(json.dumps(manifest, indent=2, ensure_ascii=False)+'\n', encoding='utf-8')
    (root/'assets/extension-public-key.der').write_bytes(key)
key = base64.b64decode(manifest['key'], validate=True)
assert key == (root/'assets/extension-public-key.der').read_bytes(), 'Manifest and Rust identity differ'
digest = hashlib.sha256(key).hexdigest()[:32]
identity = ''.join(chr(ord('a')+int(n,16)) for n in digest)
print(f'Extension public key and Rust identity match: {identity}')
