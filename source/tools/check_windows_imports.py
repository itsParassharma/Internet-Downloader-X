"""Inspect release PE imports so standalone packages do not need the VC++ redistributable."""
import argparse
import hashlib
import json
from pathlib import Path
import struct

SOURCE = Path(__file__).resolve().parents[1]

def imports(path):
    data = path.read_bytes()
    pe = struct.unpack_from('<I', data, 0x3c)[0]
    assert data[pe:pe+4] == b'PE\0\0', f'Not a PE file: {path}'
    sections = struct.unpack_from('<H', data, pe+6)[0]
    optional_size = struct.unpack_from('<H', data, pe+20)[0]
    optional = pe+24
    magic = struct.unpack_from('<H', data, optional)[0]
    assert magic in (0x10b, 0x20b)
    directory = optional + (112 if magic == 0x20b else 96)
    import_rva = struct.unpack_from('<I', data, directory+8)[0]
    section_table = optional+optional_size
    def offset(rva):
        for index in range(sections):
            virtual_size, virtual_address, raw_size, raw_offset = struct.unpack_from('<4I', data, section_table+40*index+8)
            if virtual_address <= rva < virtual_address+max(virtual_size,raw_size):
                return raw_offset+rva-virtual_address
        raise ValueError(f'Unmapped PE RVA {rva:x}')
    result = []
    if import_rva:
        position=offset(import_rva)
        while any(data[position:position+20]):
            name=offset(struct.unpack_from('<I', data, position+12)[0])
            result.append(data[name:data.index(b'\0',name)].decode('ascii'))
            position+=20
    return sorted(result, key=str.lower)

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--release-dir',type=Path,default=SOURCE/'target/release')
    args=parser.parse_args()
    report={}
    for name in ('idx.exe','idx-native-host.exe'):
        path=args.release_dir/name
        dlls=imports(path)
        external=[dll for dll in dlls if dll.lower().startswith(('vcruntime','msvcp','ucrtbase','api-ms-win-crt-'))]
        assert not external, f'{name} still requires separate C runtime DLLs: {external}'
        report[name]={'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'imports':dlls,'separate_vc_runtime_required':False}
    (SOURCE/'docs/BINARY-DEPENDENCIES.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
    print('Both release binaries pass PE import checks: no separate VC++ or UCRT runtime DLL required.')

if __name__=='__main__':
    main()
