# IDX 0.4.0 validation - 17 October 2026

The locked Cargo dependencies compile. This release adds bounded recursive Grabber scanning and cancellation using the existing native wizard, and makes the Chrome upload archive unmistakable. Earlier releases retain their own validation reports.

## Executed checks

- Rust 1.99.1, Windows x64 MSVC, Visual Studio 2022, SDK 10.0.26100.0.
- `cargo fmt --all -- --check`: passed.
- `cargo test --locked`: **74 application + 8 native-host tests passed**. Ten crawler tests and a controller/persistence test exercise real local HTTP fixtures, internal/external depth, redirect scope, cycles, parent directories, request/HTML limits, base URLs/entities, failed pages, cancellation during stalled headers/body reads and per-file source-page persistence. One crawler redirect test existed previously; ten tests are new in this revision.
- A new test fixture initially inherited nonblocking sockets on Windows, causing intermittent WouldBlock panics and a double-panic process abort during cleanup. Explicit blocking mode on accepted fixture sockets and unwind-safe cleanup fixed it. Three full parallel runs passed afterward; a final full run also passed after adding the external-to-base redirect depth case. This was a test-fixture defect, not a demonstrated application crash.
- `cargo build --release --locked --bins`: passed without warnings.
- Forty extension automatic-capture mock scenarios and twelve native-protocol/provenance checks pass.
- Migration audit: all 18,979 references remain valid; 26 partial correspondences, 18,953 unmapped and zero verified complete individual ports.
- All twelve original column definitions match original binary evidence. Grabber template bytes remain unchanged; original control IDs, fonts, geometry and artwork are reused. Prior About/batch branding text edits remain as documented.

## Real browser and installation

RELEASE-ACCEPTANCE.json records the successful isolated Chromium/native-host/download run on the exact release binaries. Chrome for Testing 156.0.8078.4 / Playwright Chromium 1248 used an Inno Setup 6.7.3 installer with a test-only AppId. The run preserved existing shortcuts, restored native-host registrations and uninstalled its isolated app while retaining test downloads/library.

An ordinary IDX launch automatically configured the bundled Chrome/Edge identity; the installer made no browser registration changes and requested no ID. The copied extension retained its stable public-key identity. Real native messaging reported IDX 0.4.0. The popup has no ID field or Copy ID button, fits 401 pixels without horizontal overflow and produced no JavaScript errors.

Two 212,992-byte downloads traversed Chromium, the native host, durable inbox and Rust transfer engine. One used its custom extension/site category folder; the unmatched file used the default folder. Both matched the fixture exactly and retained source-page provenance. Live browser download-event permission/cancellation remains mock-tested; Edge manifests are checked, while live Edge/Firefox acceptance remains pending.

## Desktop access and equivalence limits

Computer Use was retried against the installed app. It returned `Computer Use was not approved to use IDX - Internet Downloader X` at launch despite existing user consent. App/window enumeration worked; the settings interface did not resolve the locally available Computer Use installation. The exact underlying permission setting is not exposed here. No alternate desktop inspection bypass was attempted.

Native Grabber/Columns/Scheduler/category/first-launch guide visual acceptance and exact original-app comparison therefore remain unverified. The browser screenshots are real Chromium observations, not evidence of desktop equivalence. The crawler is a tested independent Rust implementation, not a complete translation of the recovered crawl engine. Main-domain expansion, JavaScript, authentication, advanced filters, saved projects and offline rewriting remain absent.

## Packages

Upload **UPLOAD-TO-CHROME-WEB-STORE-IDX-0.4.0.zip** to the Chrome Developer Dashboard. It is byte-identical to **IDX-Chrome-Extension-0.4.0.zip**, with `manifest.json` at its root and exactly twelve runtime files. The packager validates the ZIP CRC and runtime allowlist; no executable, PowerShell script or private key is inside. **IDX-Chrome-Listing-Assets-0.4.0.zip** contains images/text, and older Store Kit ZIPs also contain no extension manifest. Portable and source archives have different roots and are not extension upload packages.

The publisher must synchronize the Chrome draft public key and rebuild/retest the desktop/extension pair before publication. Public installer/privacy/support URLs remain publisher details. End users never copy IDs. This tooling did not upload or publish a listing.

`python tools/package_release.py --include-source --publish-root` reproduces the build, audits, checks and packages. `--skip-build` is used only after testing the identical release binaries. CARGO_HOME is `C:\Users\PRS\Desktop\Decomps\.build-cache\cargo`. Tokio was already in Cargo.lock; making its cached runtime an explicit dependency added no new package/version. Static C-runtime linking remains enabled; BINARY-DEPENDENCIES.json records Windows OS-only imports.

All 18,979 individual ports remain the target, not a completed claim. See MIGRATION-STATUS.md and the complete FUNCTION-MIGRATION.csv ledger for remaining work.
