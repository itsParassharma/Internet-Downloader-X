# Reverse-engineering evidence and rebuild boundary

Snapshot: 13 October 2026. IDX is a new Rust implementation informed by the supplied application's interface and workflows. It is not a completed function-by-function port. Decompilation exports are analysis material, not original or compilable source. The original executable was not run for this inventory.

## Source material

- Original extracted binary: `C:\Users\Paras\Desktop\idman643build15-decompiled\extracted\IDMan.exe`.
- SHA-256 of the inspected `IDMan.exe`: `e3fc655a335e17a6fdd09a9b315291d0c8863180f0d801e5dd7433c9e56afbd8`.
- PE menus, dialogs and string tables were parsed with LIEF plus a format-aware reader. All cited resource IDs are decimal, language 1033; offsets are original executable **file offsets**, not virtual addresses.
- English translation source: `extracted/Languages/template.lng`. Comments and text are evidence of interface intent, not executable instructions.
- Export: `decompiled/IDMan/function-index.csv`, `IDMan-all.c`, and one `0xADDRESS.c` file per recovered entry. Existing REA evidence ID for sampled export rows: `ev_1cb5db97021bc0dcb9f161745dbba24bb75d5213ceafba8f008dd099f55c163e`. The classification below is a separate local analysis of that exported index.
- CHM files (`idman.chm`, `grabber.chm`, `scheduler.chm`, `tutor.chm`) were inventoried but not decoded in this pass. Their presence alone was not used as proof of behavior.
- Companion DLLs, browser extensions, drivers and helpers are shipped separately. The main-executable export does not establish that every routine in these components has been decompiled.

## What “18,979 functions” means

The supplied index contains **18,979 entries**, all marked `ok`, with 18,979 individual pseudocode files. This verifies export coverage of the recovered index. It does not establish recovery of every original authored function, original names/types, all runtime-generated paths, correct semantics, or successful Rust ports.

Conservative, non-overlapping name-based categories:

| Export category | Entries |
| --- | ---: |
| Unnamed FUN functions; application/library role unknown | 7,314 |
| Named Catch exception fragments | 1,428 |
| Thunk-labelled candidates (not assumed trivial) | 109 |
| Other named functions; unclassified conservatively | 906 |
| Explicit MFC/ATL/runtime symbol candidates | 1,210 |
| Explicit exception-runtime symbol candidates | 9 |
| Named Unwind cleanup fragments | 7,865 |
| Named delayed-import wrappers | 111 |
| Named DLL import stubs | 27 |
| **Total** | **18,979** |

The **9,293 Catch/Unwind fragments** are associated with compiler exception machinery. Some contain application-specific error reporting or cleanup, so they must not simply be deleted from a faithful behavioral analysis. They are not 9,293 independently designed user features. The 1,210 explicit framework/runtime candidates are recognized only from a fixed whitelist of class and namespace names; 906 other named routines remain unclassified, including additional likely library functions. The 7,314 `FUN_` routines likewise may mix application logic and unrecognized libraries. No exact proprietary-function count or port-completion percentage is defensible from these names.

Examples: `0x611A97` is named `CWnd::GetDlgItem`, `0x635542` is named `ATL::CRowset<ATL::CAccessorBase>::ReleaseRows`, and `0x65D4D3` is named `WS2_32.DLL::recv`. A Rust/egui/reqwest replacement uses different framework code and need not imitate these routines one-for-one. Conversely, `0x471D20`, although thunk-labelled, exports a substantial body with state changes and window messages: the label alone does not prove it is a disposable wrapper.

Application diagnostic anchors remain useful: the string at `0x460856` names `CDownloaderDlg::OnColumnclickUrlList`; `0x460C38` names `CreateTreePopupMenu`; `0x46656E` names `OnProperties`. These are diagnostic references inside catch fragments, not independently verified addresses of the named handlers. Use caller/range analysis before assigning original method names.

## Interface anchors

| Original resource | Original file offset | Evidence and rebuild role |
| --- | --- | --- |
| RT_MENU 129 | `0x39F360` | Tasks, File, Downloads, View, Help, Registration; main workflow skeleton |
| RT_DIALOG 102 | `0x38E850` | Main Internet Download Manager window |
| RT_DIALOG 130 / 135 / 148 | `0x38CEB8` / `0x38D488` / `0x38D270` | Add download, file properties, address entry |
| RT_DIALOG 103 / 363–367 | `0x38CC48` / `0x39CF28` onward | Transfer progress, per-file speed, completion options |
| RT_DIALOG 280 | `0x3967F0` | Sequential numeric/alphabetic batch generation |
| RT_DIALOG 306 / 307 / 308 | `0x399E58` / `0x39A668` / `0x39A818` | Schedule, queue contents/concurrency, queue management |
| RT_DIALOG 145 / 146 | `0x38E9E8` / `0x38EB78` | Website/server credentials; functional download authentication |
| RT_DIALOG 140 / 369 | `0x38F9F8` / `0x39DB80` | Proxy, SOCKS, protocol-specific credentials |
| RT_DIALOG 138 | `0x38C900` | Product registration form; intentionally omitted from IDX |
| RT_DIALOG 100 | `0x38D988` | About screen with product purchase/registration fields |

Product registration must be distinguished from website access. Dialog 138 collects a product serial number and registration identity. Dialogs 145/146, grabber dialog 257, and proxy dialog 369 provide real server authentication. IDX omits product licensing, activation and account requirements while retaining optional transfer credentials. HTTPS certificate checks and optional file checksums are integrity features, not product activation.

Selected original icons and UI resources are reused at the user's explicit request to match the original appearance. The user asserted rights to modify and reuse the supplied application. Those original assets retain their original attribution and are separate from the MIT license on new Rust code. See `../assets/RESOURCE-NOTICE.txt` and the asset extraction inventory for individual source locations.

## Reproducibility and verification boundary

`original-ui-inventory.json` records parsed original menu/dialog/string-table structures. `function-classification.csv` records every index entry and its conservative classification. `function-stats.json` records counts and the index SHA-256. `app-diagnostic-anchors.json` records 69 application-named diagnostic string references; these require deeper tracing before being treated as method mappings.

The feature matrix compares static original evidence against readable Rust source (`model.rs`, `ui.rs`, `engine.rs`, `grabber.rs`) and browser-integration documentation. It does not certify runtime parity, speed parity, interoperability with original databases, or a completed port. Build/test results should be reported separately by the release process. Source may change after this snapshot; update the matrix after integrating new features.
