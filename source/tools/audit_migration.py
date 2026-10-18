"""Reconcile the full recovered-function inventory with reviewed Rust mappings.

Uses only Python's standard library. Run from any directory. Reference archives
and original indexes are read only. The generated ledger never infers a function
port from a matching name, a recovered file, or a routed dialog.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import sys
import zipfile
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def read_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def read_csv(path: Path):
    with path.open(encoding="utf-8-sig", newline="") as stream:
        return list(csv.DictReader(stream))


def sha256(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def audit(reference: Path) -> tuple[dict, list[dict]]:
    docs = ROOT / "docs"
    index_path = reference / "function-index.csv"
    archive_path = reference / "All-18979-functions.zip"
    index = read_csv(index_path)
    classifications = read_csv(docs / "function-classification.csv")
    classes = {row["address"].lower(): row for row in classifications}
    mappings_list = read_json(docs / "FUNCTION-MAPPINGS.json")["mappings"]
    mappings = {row["address"].lower(): row for row in mappings_list}
    addresses = {row["address"].lower() for row in index}
    errors = []
    if len(addresses) != len(index):
        errors.append("Duplicate addresses in the recovered function index.")
    if len(classes) != len(classifications) or set(classes) != addresses:
        errors.append("Classification addresses do not match the function index exactly.")
    if len(mappings) != len(mappings_list) or not set(mappings).issubset(addresses):
        errors.append("Reviewed mappings contain duplicate or unknown addresses.")
    for mapping in mappings_list:
        if mapping["status"] != "partial_workflow_correspondence":
            errors.append(f"Unsupported mapping status: {mapping['address']}")
        for symbol in mapping["rust_symbols"]:
            filename, name = symbol.split("::", 1)
            source = ROOT / filename
            if not source.is_file() or f"fn {name}(" not in source.read_text(encoding="utf-8"):
                errors.append(f"Mapped Rust symbol missing: {symbol}")

    ledger = []
    with zipfile.ZipFile(archive_path) as archive:
        members = archive.infolist()
        expected = {f"{address}.c" for address in addresses}
        actual = {entry.filename for entry in members}
        if len(members) != len(actual) or actual != expected:
            errors.append("Reference archive entries do not match the function index exactly.")
        for row in index:
            address = row["address"].lower()
            filename = f"{address}.c"
            valid = False
            try:
                # Reading to EOF verifies each member's CRC.
                body = archive.read(filename)
                valid = bool(body.strip())
            except (KeyError, zipfile.BadZipFile) as error:
                errors.append(f"Cannot validate {filename}: {error}")
            if not valid:
                errors.append(f"Empty or invalid pseudocode: {filename}")
            classification = classes.get(address, {})
            if classification.get("name") != row["name"]:
                errors.append(f"Classification name differs at {address}.")
            mapping = mappings.get(address, {})
            ledger.append({
                "address": address,
                "name": row["name"],
                "classification": classification.get("static_classification", "missing"),
                "decompilation_status": row["decompilation_status"],
                "reference_crc_valid_and_nonempty": valid,
                "rust_status": mapping.get("status", "unmapped_reference"),
                "rust_symbols": "; ".join(mapping.get("rust_symbols", [])),
                "remaining": mapping.get("remaining", "Review original behavior and map to a Rust workflow; no individual port is established."),
            })

    extraction = read_json(docs / "NATIVE-UI-EXTRACTION.json")
    original_resources = []
    for resource in extraction["extracted_resources"]:
        path = reference / "native-ui-resources" / resource["file"]
        matches = path.is_file() and sha256(path) == resource["sha256"]
        original_resources.append({"file": resource["file"], "matches_recorded_sha256": matches})
        if not matches:
            errors.append(f"Original resource differs or is missing: {resource['file']}")

    derivation = read_json(docs / "NATIVE-UI-DERIVATION.json")
    derived_resources = []
    for dialog in derivation["dialogs"]:
        if "derived_sha256" not in dialog:
            continue
        filename = f"dialog_{dialog['id']}.bin"
        path = ROOT / "assets" / "native" / filename
        matches = path.is_file() and sha256(path) == dialog["derived_sha256"]
        derived_resources.append({"file": filename, "matches_recorded_sha256": matches})
        if not matches:
            errors.append(f"Derived template differs or is missing: {filename}")

    coverage = read_json(docs / "NATIVE-UI-COVERAGE.json")
    stats = {
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "source_revision": "2026-10-17 IDX 0.7.0 automatic browser pickup and direct-media panels",
        "integrity_ok": not errors,
        "errors": errors,
        "function_index_sha256": sha256(index_path),
        "reference_archive_sha256": sha256(archive_path),
        "indexed_functions": len(index),
        "classification_counts": dict(sorted(Counter(row["classification"] for row in ledger).items())),
        "rust_status_counts": dict(sorted(Counter(row["rust_status"] for row in ledger).items())),
        "verified_complete_function_ports": 0,
        "complete_idm_feature_parity": False,
        "port_count_note": "Zero verified complete individual ports does not mean zero working features. The app has independent implementations; reviewed correspondences are partial.",
        "recorded_native_routes": coverage["routed_native_templates"],
        "recorded_native_routes_source_date": coverage["date"],
        "extracted_dialogs": derivation["extracted_dialogs"],
        "derived_layouts": len(derived_resources),
        "original_resource_checks": original_resources,
        "derived_template_checks": derived_resources,
        "rust_source_sha256": {str(path.relative_to(ROOT)).replace("\\", "/"): sha256(path)
                              for path in sorted((ROOT / "src").rglob("*.rs"))},
        "validation_note": "This audit checks reference integrity and mapped symbol existence. It does not compile Rust or prove runtime/UI equivalence. See BUILD-VALIDATION.md for executed release tests and remaining desktop visual acceptance.",
    }
    return stats, ledger


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference", type=Path, default=ROOT.parent / "reverse-engineering")
    parser.add_argument("--check", action="store_true", help="Validate without writing the generated ledger and summary")
    args = parser.parse_args()
    stats, ledger = audit(args.reference)
    if not args.check:
        (ROOT / "docs" / "MIGRATION-AUDIT.json").write_text(
            json.dumps(stats, indent=2) + "\n", encoding="utf-8")
        with (ROOT / "docs" / "FUNCTION-MIGRATION.csv").open("w", newline="", encoding="utf-8") as stream:
            writer = csv.DictWriter(stream, fieldnames=list(ledger[0]))
            writer.writeheader()
            writer.writerows(ledger)
    print(json.dumps({key: stats[key] for key in (
        "integrity_ok", "errors", "indexed_functions", "classification_counts",
        "rust_status_counts", "verified_complete_function_ports", "recorded_native_routes",
        "extracted_dialogs", "derived_layouts")}, indent=2))
    return 0 if stats["integrity_ok"] else 1


if __name__ == "__main__":
    sys.exit(main())
