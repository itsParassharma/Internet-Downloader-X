"""Build and package IDX without requiring PowerShell scripts or policy changes.

Python 3.11+, Cargo, Node.js and Inno Setup are development dependencies.
The shipped extension is plain Manifest V3; IDX registers its host on app launch.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
import zipfile
from datetime import datetime

SOURCE = Path(__file__).resolve().parents[1]
PROJECT = SOURCE.parent
DIST = SOURCE / "dist"
VERSION = tomllib.loads((SOURCE / "Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
EXTENSION_FILES = ["manifest.json", "background.js", "native.js", "popup.html", "popup.css", "popup.js",
                   "privacy.html", "privacy.css", "media.js", "media-background.js", "media-content.js", *[f"icons/idx-{size}.png" for size in (16,32,48,128)]]

def run(*args):
    subprocess.run([str(arg) for arg in args], cwd=SOURCE, check=True)

def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()

def copy_file(source, destination):
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)

def archive(directory, destination, prefix=""):
    temporary = destination.with_suffix(".zip.tmp")
    with zipfile.ZipFile(temporary, "w", zipfile.ZIP_DEFLATED, compresslevel=9, strict_timestamps=False) as out:
        for file in sorted(directory.rglob("*")):
            if file.is_file():
                out.write(file, (Path(prefix) / file.relative_to(directory)).as_posix())
    with zipfile.ZipFile(temporary) as check:
        assert check.testzip() is None
    temporary.replace(destination)

def compiler(explicit):
    candidates = [explicit, os.environ.get("IDX_ISCC"), shutil.which("ISCC.exe"),
        str(Path(os.environ.get("LOCALAPPDATA", "")) / "Programs/Inno Setup 6/ISCC.exe"),
        str(Path(os.environ.get("ProgramFiles(x86)", "")) / "Inno Setup 6/ISCC.exe")]
    for candidate in candidates:
        if candidate and Path(candidate).is_file():
            return candidate
    raise SystemExit("Inno Setup compiler missing. Supply --iscc or use --skip-installer.")

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--skip-build", action="store_true", help="Package an already tested release")
    parser.add_argument("--skip-installer", action="store_true")
    parser.add_argument("--release-dir", type=Path, default=SOURCE / "target/release")
    parser.add_argument("--iscc")
    parser.add_argument("--publish-root", action="store_true", help="Back up and refresh the user's release folder")
    parser.add_argument("--include-source", action="store_true", help="Also archive source and the supplied references")
    args = parser.parse_args()
    if not args.skip_build:
        run("cargo", "fmt", "--all", "--", "--check")
        run("cargo", "test", "--locked")
        run("cargo", "build", "--release", "--locked", "--bins")
    run(sys.executable, "tools/fetch_media_tools.py")
    for test in ("automatic-capture", "native-protocol", "media", "media-background", "browser-session"):
        run("node", f"browser-extension/tests/{test}.test.cjs")
    run(sys.executable, "tools/audit_migration.py")
    run(sys.executable, "tools/recover_columns.py")
    run(sys.executable, "tools/sync_extension_identity.py", "--check")
    release = args.release_dir.resolve()
    for name in ("idx.exe", "idx-native-host.exe"):
        if not (release / name).is_file():
            raise SystemExit(f"Missing release binary: {release / name}")
    run(sys.executable, "tools/check_windows_imports.py", "--release-dir", release)
    manifest = json.loads((SOURCE / "browser-extension/chromium/manifest.json").read_text(encoding="utf-8"))
    assert manifest["version"] == VERSION and manifest["manifest_version"] == 3
    DIST.mkdir(exist_ok=True)
    artifacts = []
    with tempfile.TemporaryDirectory(prefix="package-", dir=DIST) as temporary:
        stage = Path(temporary)
        assert stage.resolve().is_relative_to(DIST.resolve()), "Temporary package must stay inside dist"
        portable = stage / f"IDX-{VERSION}-portable"
        for name in ("idx.exe", "idx-native-host.exe"):
            copy_file(release / name, portable / name)
        for name in ("LICENSE", "README.md", "THIRD-PARTY-NOTICES.txt"):
            copy_file(SOURCE / name, portable / name)
        copy_file(SOURCE / "assets/RESOURCE-NOTICE.txt", portable / "RESOURCE-NOTICE.txt")
        shutil.copytree(SOURCE / "third-party-licenses", portable / "third-party-licenses")
        shutil.copytree(SOURCE / "media-tools", portable / "media-tools")
        shutil.copytree(SOURCE / "docs", portable / "docs")
        extension = stage / "extension"
        for name in EXTENSION_FILES:
            copy_file(SOURCE / "browser-extension/chromium" / name, extension / name)
            copy_file(extension / name, portable / "browser-extension/chromium" / name)
        # The public key stabilizes unpacked development installs. Chrome Web Store
        # assigns its own identity and rejects this field in a submitted package.
        store_manifest = dict(manifest)
        store_manifest.pop("key", None)
        (extension / "manifest.json").write_text(
            json.dumps(store_manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        extension_zip = DIST / f"IDX-Chrome-Extension-{VERSION}.zip"
        archive(extension, extension_zip)
        with zipfile.ZipFile(extension_zip) as check:
            assert sorted(check.namelist()) == sorted(EXTENSION_FILES)
            uploaded_manifest = json.loads(check.read("manifest.json"))
            assert "key" not in uploaded_manifest, "Store uploads must omit the development public key"
            assert uploaded_manifest == store_manifest
            assert all(not name.endswith((".exe", ".ps1", ".pem")) for name in check.namelist())
        artifacts.append(extension_zip)
        upload_zip = DIST / f"UPLOAD-TO-CHROME-WEB-STORE-IDX-{VERSION}.zip"
        copy_file(extension_zip, upload_zip)
        assert digest(upload_zip) == digest(extension_zip)
        artifacts.append(upload_zip)
        portable_zip = DIST / f"IDX-{VERSION}-portable.zip"
        archive(portable, portable_zip, portable.name)
        with zipfile.ZipFile(portable_zip) as check:
            unpacked_manifest = json.loads(check.read(f"{portable.name}/browser-extension/chromium/manifest.json"))
            assert unpacked_manifest == manifest, "Unpacked installs must retain their stable identity"
            for name in ("idx.exe", "idx-native-host.exe"):
                assert hashlib.sha256(check.read(f"{portable.name}/{name}")).hexdigest() == digest(release / name)
        artifacts.append(portable_zip)
    if not args.skip_installer:
        definitions = [f"/DBuildOutputDir={release}"]
        run(compiler(args.iscc), "/Qp", *definitions, SOURCE / "installer/idx.iss")
        artifacts.append(DIST / f"IDX-Setup-{VERSION}.exe")
    store = SOURCE / "browser-extension/store"
    if store.is_dir():
        store_zip = DIST / f"IDX-Chrome-Listing-Assets-{VERSION}.zip"
        archive(store, store_zip)
        artifacts.append(store_zip)
    if args.include_source:
        source_zip = DIST / f"IDX-{VERSION}-source-and-reference.zip"
        temporary_zip = source_zip.with_suffix(".zip.tmp")
        with zipfile.ZipFile(temporary_zip, "w", zipfile.ZIP_DEFLATED, compresslevel=9, strict_timestamps=False) as out:
            for directory, prefix in [(SOURCE, "source"), (PROJECT / "reverse-engineering", "reverse-engineering")]:
                for current, folders, files in os.walk(directory):
                    folders[:] = sorted(name for name in folders if name not in
                        {"target", "dist", ".migration-backup", ".git", "__pycache__", "node_modules"})
                    for name in sorted(files):
                        file = Path(current) / name
                        if file.parent == SOURCE and file.suffix in {".exe", ".pdb"}:
                            continue
                        out.write(file, (Path(prefix) / file.relative_to(directory)).as_posix())
            for name in ("AGENTS.md",):
                out.write(PROJECT / name, name)
        with zipfile.ZipFile(temporary_zip) as check:
            assert check.testzip() is None
        temporary_zip.replace(source_zip)
        artifacts.append(source_zip)
    for binary in ("idx.exe", "idx-native-host.exe"):
        copy_file(release / binary, DIST / binary)
        artifacts.append(DIST / binary)
    sums = "".join(f"{digest(file)}  {file.name}\n" for file in artifacts)
    (DIST / "SHA256SUMS.txt").write_text(sums, encoding="utf-8")
    if args.publish_root:
        backup = SOURCE / ".migration-backup" / ("pre-release-" + datetime.now().strftime("%Y%m%d-%H%M%S"))
        for file in [*artifacts, DIST / "SHA256SUMS.txt"]:
            destination = PROJECT / file.name
            if destination.exists():
                copy_file(destination, backup / file.name)
            copy_file(file, destination)
        copy_file(SOURCE / "README.md", PROJECT / "README.md")
        shutil.copytree(SOURCE / "media-tools", PROJECT / "media-tools", dirs_exist_ok=True)
        shutil.copytree(SOURCE / "docs", PROJECT / "docs", dirs_exist_ok=True)
        shutil.copytree(SOURCE / "browser-extension/chromium", PROJECT / "browser-extension/chromium", dirs_exist_ok=True)
        if store.is_dir():
            shutil.copytree(store, PROJECT / "browser-extension/store", dirs_exist_ok=True)
        print(f"Release folder updated; previous files retained in {backup}")
    print(sums)

if __name__ == "__main__":
    main()
