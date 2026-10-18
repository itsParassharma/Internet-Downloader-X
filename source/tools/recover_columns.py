"""Reproduce original column names and Reset defaults from the verified reference."""
import hashlib
import json
from pathlib import Path
import re
import struct
import zipfile

ROOT = Path(__file__).resolve().parents[1]
REFERENCE = ROOT.parent / "reverse-engineering"
binary = (REFERENCE / "extracted-original/IDMan.exe").read_bytes()
sha = hashlib.sha256(binary).hexdigest()
assert sha == "e3fc655a335e17a6fdd09a9b315291d0c8863180f0d801e5dd7433c9e56afbd8"
u16 = lambda offset: struct.unpack_from("<H", binary, offset)[0]
u32 = lambda offset: struct.unpack_from("<I", binary, offset)[0]
pe = u32(60)
image_base = u32(pe + 24 + 28)
section_table = pe + 24 + u16(pe + 20)
sections = []
for index in range(u16(pe + 6)):
    offset = section_table + index * 40
    sections.append((u32(offset + 12) + image_base, u32(offset + 16), u32(offset + 20)))


def file_offset(address):
    for va, size, raw in sections:
        if va <= address < va + size:
            return raw + address - va
    raise ValueError(f"Address has no file bytes: {address:#x}")


def virtual_address(offset):
    for va, size, raw in sections:
        if raw <= offset < raw + size:
            return va + offset - raw
    raise ValueError(f"Offset has no virtual address: {offset:#x}")


with zipfile.ZipFile(REFERENCE / "All-18979-functions.zip") as archive:
    def routine(address):
        names = [n for n in archive.namelist() if n.endswith(f"{address}.c")]
        assert len(names) == 1
        return archive.read(names[0]).decode("utf-8")
    reset = routine("0x4aab20")
    labels = routine("0x4aa630")

# Switch cases in 0x4aa630 select these CString globals by original column ID.
globals_by_id = [0x78a018, 0x78a030, 0x78a014, 0x78a020, 0x78a01c, 0x78a024,
                 0x78a028, 0x78a034, 0x78a02c, 0x78a038, 0x78a03c, 0x78a954]
visible = {int(value, 0) for value in re.findall(
    r"SendMessageA\([^;]+,0x102b,(0x[0-9a-f]+|[0-9]+),", reset)}
widths = {int(offset, 0): int(value, 0) for offset, value in re.findall(
    r"\*\(undefined4 \*\)\(in_ECX \+ (0x[0-9a-f]+|[0-9]+)\) = (0x[0-9a-f]+|[0-9]+);", reset)}
columns = []
for column_id, global_address in enumerate(globals_by_id):
    assert f"DAT_{global_address:08x}" in labels
    # push <ASCII string>; mov ecx,<CString>; call <assignment>.
    pattern = b"\x68(.{4})\xb9" + re.escape(struct.pack("<I", global_address)) + b"\xe8"
    candidates = list(re.finditer(pattern, binary, re.DOTALL))
    assert len(candidates) == 1, (column_id, len(candidates))
    match = candidates[0]
    string_address = struct.unpack("<I", match.group(1))[0]
    offset = file_offset(string_address)
    title = binary[offset:binary.index(b"\0", offset)].decode("ascii")
    columns.append({
        "original_id": column_id, "title": title,
        "visible": column_id in visible, "width_96dpi": widths[0x184 + 4 * column_id],
        "label_global": hex(global_address), "string_address": hex(string_address),
        "string_file_offset": hex(offset), "initializer_address": hex(virtual_address(match.start())),
        "initializer_bytes": binary[match.start():match.start() + 15].hex(),
    })
report = {"binary_sha256": sha, "label_function": "0x4aa630", "reset_function": "0x4aab20",
          "method": "CString initializer instructions and ASCII payload; recovered Reset assignments",
          "runtime_visual_verification": False, "columns": columns}
# Compare the actual Rust defaults against this independent binary-derived oracle.
# IDs in old JSON libraries remain stable; only their presentation order changes.
rust = (ROOT / "src/columns.rs").read_text(encoding="utf-8")
default_source = rust.split("pub const DEFAULTS:", 1)[1].split("= [", 1)[1].split("];", 1)[0]
title_source = rust.split("pub const TITLES:", 1)[1].split("= [", 1)[1].split("];", 1)[0]
titles = re.findall(r'"([^"\\]*)"', title_source)
defaults = [(int(i), v == "true", int(w)) for i, v, w in re.findall(
    r"\((\d+), (true|false), (\d+)\)", default_source)]
assert [(titles[i], v, w) for i, v, w in defaults] == [
    (c["title"], c["visible"], c["width_96dpi"]) for c in columns
], "Rust definitions differ from the verified original"
report["rust_defaults_match"] = True
(ROOT / "docs/ORIGINAL-COLUMNS.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
print(f"Verified {len(columns)} original column labels, order, visibility and widths against Rust defaults.")
