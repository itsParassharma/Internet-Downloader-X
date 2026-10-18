# IDX 0.2.0 validation — 17 October 2026

The missing Cargo dependencies and MSVC resource-compiler discovery are fixed. The Windows x64 release now builds using the locked dependency graph. The earlier 15 October record is preserved in BUILD-VALIDATION-2026-10-15.md and is historical evidence only.

## Executed checks

- Rust 1.99.1, target x86_64-pc-windows-msvc, Visual Studio 2022 C++ tools and Windows SDK 10.0.26100.0.
- `cargo fmt --all -- --check`: passed.
- `cargo test --locked`: **54 application tests and 8 native-host tests passed**. This includes the previously blocked scheduler, column preference, parent-page persistence and redirected Grabber integration tests.
- `cargo build --release --locked --bins`: passed without warnings for idx.exe and idx-native-host.exe.
- `.cargo/config.toml` links the C runtime statically for x86_64 MSVC. PE import checks in BINARY-DEPENDENCIES.json confirm that neither executable needs VCRUNTIME140.dll or a separate UCRT redistributable. Packaging checks this again before creating archives.
- Node tests: **40 automatic-capture scenarios and 12 connection/acknowledgement/provenance checks passed**. The capture tests exercise Chrome-style and Firefox-style API mocks; they are not live Firefox acceptance.
- Original archive, index, mappings and native resource integrity: all **18,979 references** valid; 14 partial correspondences, 18,965 individually unmapped, zero verified complete function ports.
- All 12 original English column labels and default order/visibility/widths matched the hash-verified original executable and recovered Reset routine.

## Real browser and installer acceptance

`tools/test_extension.cjs` uses Chrome for Testing 156.0.8078.4 (Playwright Chromium 1248), an isolated browser profile, separate IDX data directory, separate instance port and a test-only installer AppId. Inno Setup 6.7.3 compiled the installer. The test preserves/restores the user's native-host registry defaults and checks that the existing IDX Start Menu shortcuts are unchanged.

The Manifest V3 service worker started, the installer registered Chrome and Edge manifests, and real Chrome native messaging reported IDX 0.2.0 / protocol 2. Connection checks queued no downloads. Popup settings persisted and automatic capture stayed off by default. A real extension request with the automatic flag passed through the installed native host and durable inbox into the Rust HTTP engine. The completed 212,992-byte file matched exactly, SHA-256 `efab2ca5ea5cee742f47afe1c2b1ed4a385d44c7291d0a787cf8328a7cd1a7cd`. Its parent-page URL persisted separately from the HTTP Referer. Popup JavaScript had no page errors.

The browser test directly calls the extension handoff with the automatic flag. It does **not** establish real browser download-event permission/cancellation behavior; that path has mock coverage. Edge's manifest is validated, but live Edge and Firefox handoff have not been executed. See RELEASE-ACCEPTANCE.json for installed-binary hashes, installer/uninstaller checks and the final run's results.

The browser popup was rendered and visually inspected. The native desktop app ran and completed the transfer, but its new Columns/Scheduler UI and exact side-by-side IDM comparison are **not visually accepted**: Computer Use returned “was not approved to use IDX - Internet Downloader X”. App access was requested. No alternative desktop-automation route was used to bypass that block.

## Release and remaining limits

Version 0.2.0 packages contain the current compiled binaries, scheduler/Columns changes, independent parent-page metadata and updated browser host. SHA256SUMS.txt identifies the delivered files. The Chrome extension ZIP contains only its Manifest V3 runtime files; installation of the Windows native host and registry connection is handled by the desktop installer, with no PowerShell required by users.

The Chrome Web Store kit includes screenshots, promotional artwork, privacy HTML and draft listing/reviewer instructions. Nothing has been uploaded or published. Publisher download/support/privacy URLs and the assigned store ID still need to be supplied after creating the listing draft.

Full IDM functionality, all 18,979 individual ports, original database compatibility, every native control, all DPI/theme settings and exhaustive restart/failure cases remain unverified or incomplete. Original resource geometry and hashes are preserved; that alone does not prove exact rendered UI equivalence.

## Reproduction

From `source/`, run `python tools/package_release.py` with Python 3.11+, Node.js, Rust, C++ build tools, the Windows SDK and Inno Setup installed. It runs tests, builds both binaries and writes the portable/installer/extension packages. Use `--skip-build` only after the same source has been tested and compiled; use `--include-source` to archive the supplied references as well. `--publish-root` backs up and updates the parent release folder.

The populated Cargo cache for this workspace is `C:\Users\PRS\Desktop\Decomps\.build-cache\cargo`; set CARGO_HOME to that path to reuse it. build.rs discovers rc.exe in installed Windows SDK directories; IDX_RC/RC may select an explicit compiler. No Windows execution policy or application-control settings were changed.
