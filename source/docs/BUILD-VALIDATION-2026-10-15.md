# IDX 0.1.0 validation record

Release work: 15 October 2026 (Australia/Sydney).

**Newer source revision:** the 17 October source migration has 10 passing standalone Rust scheduler tests, eight passing column-model tests, separate compilation of the actual native Columns handler/resources, a passing archive/resource audit, and a binary-derived check of all twelve original column labels and defaults. Full Cargo compilation, library integration tests and native UI acceptance are blocked by unavailable dependencies/network access. See [MIGRATION-STATUS.md](MIGRATION-STATUS.md). The release evidence below describes the older packaged binaries.

## Scope

The Windows main window instantiates original RT_DIALOG 102 and the original menu structure, with Registration removed. Download forms, nine Options pages, Scheduler, Grabber and the separate progress window use extracted native templates. See NATIVE-UI-COVERAGE.md: 105 extracted dialogs, 100 rebranded derived layouts, 33 runtime routes, 67 derived layouts without routes and five omitted registration/updater layouts.

Templates contain geometry, fonts, control IDs, captions and styles. New Rust handlers supply behavior. Some routed controls are disabled or adapted because the corresponding feature is unavailable. All 18,979 recovered functions remain reference pseudocode; they have not all been mapped or implemented in Rust. IDX has no product account, activation, serial key, registration or trial period. Optional server credentials are a separate download feature. Original resource copyright remains separate from the MIT license on new Rust code.

## Native main-window revision

- Release build completed without warnings with Rust 1.99.0, the locked dependency graph and LLVM-MinGW MSVCRT for Windows x64. Formatting completed.
- The extracted main window opened with original 3D toolbar/category icons, native list/tree, menu structure and IDX logo. No Registration menu appeared.
- Options General/Connection and Scheduler opened with native controls and closed normally.
- A real transfer was added through original Add URL 148 and File Info 130. The supplemental IDX More dialog accepted an expected SHA-256. Start Download opened the separate original progress layout; the main list reported live progress and transfer rate.
- Eight connections completed a 16 MiB file. Original completion dialog 286 appeared. Both IDX's persisted verification and an independent filesystem hash matched `341aacac661ccb210720bedaa9ead5d668fe5ea41a73532fc147c71e34040df1`.
- All four Grabber wizard pages opened. A local HTML page produced two results. That check exposed blank result rows caused by original LVS_OWNERDATA; the copied results template now clears that flag so IDX can store row text directly.
- Normal closing of completion, progress, Grabber and main windows was observed; the app exited normally.

After these checks, the Options host was widened to fit all original pages, Connection changed to checked/disabled "Secure HTTPS downloads", original IMAGE384 was added to Grabber, and the stored-row fix was applied. The final release built successfully and its main window was observed. The last Grabber visual check was interrupted by disappearing/unavailable windows. A monitored process returned exit code 0, with empty stderr and no application-fault record. This did not establish an artwork crash. Main shutdown paths and GDI+ declarations, structures and image/stream lifetimes were audited. Final Grabber artwork and populated-row rendering still require a completed interactive check.

Screenshots in the companion outputs folder record the native main window, Options, Scheduler, File Info and completion dialog. Some precede the small final refinements. No screenshot establishes that every original control is implemented.

## Earlier regression and packaging evidence

- An earlier build passed 28 application tests and six native-host tests covering HTTP ranges, resume/resource changes, malformed responses, persistence recovery, filename collisions, checksums, batch expansion, selection and browser framing. These are earlier-build results, not a fresh suite pass for this native-main revision.
- All 40 Chromium/Firefox capture mock scenarios passed on 15 October before this UI migration. Extension and transfer-engine behavior were not changed here.
- Earlier progress-window tests completed 16 MiB transfers, including pause/resume, with matching hashes. Its distinct native window was dragged beyond the main window's right edge.
- The prior installer installed into an isolated folder; installed binary hashes matched. Its uninstaller returned success and retained downloads/library. The current installer is rebuilt from current binaries; installation lifecycle was not repeated for this UI revision.
- All 18,979 reference entries were present, nonempty, CRC-valid and matched the recovered index.
- Original normal/hover toolbar pixels match extraction. Disabled buttons use a grayscale derivative. Main/window/installer/extension logos use distinct IDX artwork.
- All 842 third-party license provenance records resolved with matching hashes. No dependency was added during this migration.

## Blocked or incomplete checks

- An earlier fresh debug test run was blocked before execution by Windows Application Control (OS error4551). No security settings were changed and the blocked route was not retried.
- An earlier live automatic browser-handoff command was rejected by automatic approval review with "blocked by policy", without further reason. It was not retried through another route. Real-browser automatic capture remains unverified; files and mocks are included.
- An earlier final-progress styling launch was also rejected by automatic approval review. The later substantially new native-main revision launched successfully and has separate observations above; this does not convert the older rejected check into a pass.
- Firefox is an unsigned development extension; Chrome/Edge are unpacked extensions. Browser setup is separate from desktop installation.
- All controls, failure/restart cases, DPI settings, themes and dialogs have not received exhaustive runtime tests. Full IDM feature equivalence, all 18,979 function ports and original database compatibility are not established.

## Reproduction

Use `build.ps1 -RunTests` with a configured Windows Rust toolchain and Inno Setup. Scripts do not change Windows application-control policy. Record blocked execution rather than treating compilation as runtime success. NATIVE-UI-EXTRACTION.json, NATIVE-UI-DERIVATION.json, NATIVE-UI-COVERAGE.json and VERIFICATION.json provide precise source hashes and coverage.
