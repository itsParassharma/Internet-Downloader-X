"""Recover the supplied IDM payload without running the installer.

Uses the existing, independently recorded extraction manifest. Every input and
output digest is checked before writing; existing differing files are refused.
"""
import argparse
import hashlib
import json
from pathlib import Path, PureWindowsPath
import zlib


def digest(data):
    return hashlib.sha256(data).hexdigest()


def extract(installer, manifest_path, output):
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    data = installer.read_bytes()
    if digest(data) != manifest["source_sha256"]:
        raise ValueError("Installer SHA-256 differs from the decompilation input")
    root = output.resolve()
    files = []
    seen = set()
    for record in manifest["records"]:
        relative = PureWindowsPath(record["relative_path"])
        if relative.is_absolute() or relative.drive or ".." in relative.parts:
            raise ValueError(f"Unsafe manifest path: {relative}")
        destination = root.joinpath(*relative.parts).resolve()
        if not destination.is_relative_to(root) or destination == root:
            raise ValueError(f"Path escapes output: {relative}")
        key = str(destination).casefold()
        if key in seen:
            raise ValueError(f"Duplicate output: {relative}")
        seen.add(key)
        start = record["offset"] + 32
        end = start + record["compressed_bytes"]
        if start < 32 or end > len(data):
            raise ValueError(f"Invalid compressed extent: {relative}")
        stream = zlib.decompressobj()
        payload = stream.decompress(data[start:end]) + stream.flush()
        if not stream.eof or stream.unused_data or stream.unconsumed_tail:
            raise ValueError(f"Invalid zlib stream: {relative}")
        if len(payload) != record["extracted_bytes"] or digest(payload) != record["sha256"]:
            raise ValueError(f"Payload differs from recorded evidence: {relative}")
        if destination.exists() and destination.read_bytes() != payload:
            raise ValueError(f"Refusing to replace differing file: {destination}")
        files.append((destination, payload, record))
    if len(files) != manifest["file_count"]:
        raise ValueError("Manifest file count mismatch")
    for destination, payload, _ in files:
        destination.parent.mkdir(parents=True, exist_ok=True)
        if not destination.exists():
            destination.write_bytes(payload)
    report = {
        "installer": str(installer.resolve()),
        "installer_sha256": digest(data),
        "manifest_sha256": digest(manifest_path.read_bytes()),
        "method": manifest["method"],
        "installer_executed": False,
        "file_count": len(files),
        "total_extracted_bytes": sum(len(payload) for _, payload, _ in files),
        "all_recorded_hashes_match": True,
        "files": [{"path": r["relative_path"], "bytes": len(p), "sha256": digest(p)}
                  for _, p, r in files],
    }
    (root / "extraction-verification.json").write_text(
        json.dumps(report, indent=2) + "\n", encoding="utf-8"
    )
    print(json.dumps({k: v for k, v in report.items() if k != "files"}, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("installer", type=Path)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    extract(args.installer, args.manifest, args.output)
