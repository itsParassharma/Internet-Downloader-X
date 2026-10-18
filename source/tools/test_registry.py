"""Snapshot/restore only IDX native-host default values for isolated acceptance."""
import json
from pathlib import Path
import sys
import winreg

vendors = ["Google\\Chrome", "Microsoft\\Edge"]
views = [("32", winreg.KEY_WOW64_32KEY), ("64", winreg.KEY_WOW64_64KEY)]
mode, filename = sys.argv[1:]
backup = Path(filename)
if mode == "save":
    records = []
    for vendor in vendors:
        path = f"Software\\{vendor}\\NativeMessagingHosts\\org.idx.downloader"
        for label, view in views:
            record = {"path": path, "view": label, "exists": False}
            try:
                with winreg.OpenKey(winreg.HKEY_CURRENT_USER, path, 0, winreg.KEY_READ | view) as key:
                    value, kind = winreg.QueryValueEx(key, "")
                    record.update(exists=True, value=value, kind=kind)
            except FileNotFoundError:
                pass
            records.append(record)
    backup.parent.mkdir(parents=True, exist_ok=True)
    with backup.open("x", encoding="utf-8") as stream:
        json.dump(records, stream, indent=2)
elif mode == "restore":
    for record in json.loads(backup.read_text(encoding="utf-8")):
        view = dict(views)[record["view"]]
        if record["exists"]:
            with winreg.CreateKeyEx(winreg.HKEY_CURRENT_USER, record["path"], 0, winreg.KEY_SET_VALUE | view) as key:
                winreg.SetValueEx(key, "", 0, record["kind"], record["value"])
        else:
            try:
                with winreg.OpenKey(winreg.HKEY_CURRENT_USER, record["path"], 0, winreg.KEY_SET_VALUE | view) as key:
                    winreg.DeleteValue(key, "")
            except FileNotFoundError:
                pass
            # Retain an empty key rather than remove any unknown values/subkeys.
else:
    raise SystemExit("Use save or restore")
