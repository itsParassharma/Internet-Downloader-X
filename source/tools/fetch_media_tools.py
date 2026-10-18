"""Fetch the pinned LGPL media helper; verify archive and every shipped binary."""
import hashlib,json,pathlib,sys,urllib.request,zipfile,tempfile
root=pathlib.Path(__file__).resolve().parents[1]
metadata=json.loads((root/'docs/MEDIA-TOOLS.json').read_text())
out=root/'media-tools';out.mkdir(exist_ok=True)
def digest(p):
    with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
if not all((out/name).is_file() and digest(out/name)==expected for name,expected in metadata['files'].items()):
    with tempfile.TemporaryDirectory(prefix='idx-media-') as scratch:
        archive=pathlib.Path(scratch)/'media.zip'
        urllib.request.urlretrieve(metadata['source_url'],archive)
        if digest(archive)!=metadata['archive_sha256']:sys.exit('Media helper archive checksum mismatch; no files were installed.')
        with zipfile.ZipFile(archive) as z:
            for name,expected in metadata['files'].items():
                entries=[p for p in z.namelist() if pathlib.PurePosixPath(p).name==name]
                if len(entries)!=1:sys.exit('Unexpected media helper archive structure.')
                data=z.read(entries[0])
                if hashlib.sha256(data).hexdigest()!=expected:sys.exit('Media helper file checksum mismatch.')
                (out/name).write_bytes(data)
print('Pinned LGPL media helper files verified.')
