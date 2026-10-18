# Release 0.7.0 validation — 18 October 2026

132 Rust tests (106 app, 26 native host) and 97 extension checks pass. Release MSVC compilation and formatting pass. Real Chromium acceptance verifies direct HttpOnly-cookie downloads and HLS TS, AES-128 HLS, separate fMP4 audio/video, HLS byte ranges and static DASH. Each stream produces MP4 with audio and matching decoded source video frames. Tests also check quality selection, automatic pickup/fallback, exact bytes for direct media, protected inbox credentials, scoped redirects and paused-segment reuse. The bundled FFmpeg files match the pinned upstream archive checksums.

The extension still uses a DOM/CSS panel. Exact original native-overlay rendering and broader site compatibility are not established. Live/DRM streams, multi-period DASH and HLS discontinuities remain unsupported. See RELEASE-ACCEPTANCE.json for the latest browser run and BROWSER-INTEGRATION.md for supported stream structures.

# Release 0.6.0 validation - 17 October 2026

- `cargo fmt --all -- --check`, `cargo test --locked`, and `cargo build --release --locked --bins` pass: 88 application tests plus 10 native-host tests.
- Extension checks: 44 automatic capture/fallback scenarios, 12 protocol/provenance cases, 20 media URL/name cases and 13 sender/document/toolbar/exclusion cases.
- Real Chromium acceptance loads the actual extension and installed native host, exercises normal browser download events, verifies Chrome cancellation after acknowledgement, confirms native-host failure resumes the browser copy, and clicks the floating video panel through the browser UI. Completed Rust files match the fixture bytes and preserve filename/Referer.
- Embedded/dynamic players, response-MIME discovery, live full/mini preferences and unsupported playlist handling are exercised. Reports/screenshots remain in `.migration-backup/acceptance`; the sanitized current report is RELEASE-ACCEPTANCE.json.
- Desktop Computer Use successfully opened the installed IDX main window and connection guide. Full original-app visual equivalence is still pending. Browser Use rejects `chrome://extensions/` as a disallowed protocol, so the user's normal Chrome profile has not been reloaded automatically.
- Store packaging must contain the 15 runtime files with a root manifest and no `key`. Unpacked packages retain the stable development key. Store identity synchronization remains a publisher step before publication.

The following sections retain earlier release evidence and limitations as history.

---

# IDX 0.5.0 validation - 17 October 2026

This release connects the original Grabber advanced controls and filter dialogs to Rust implementations. Cargo dependencies are available and the release binaries build. Native desktop visual acceptance remains unverified.

## Executed checks

- Rust 1.99.1, Windows x64 MSVC, Visual Studio 2022, SDK 10.0.26100.0.
- `cargo fmt --all -- --check`: passed.
- `cargo test --locked`: **86 application + 8 native-host tests passed**. Twelve new tests cover filename masks and the resolved start page, domain/path boundaries, IDN/IPv6/ports, checked size units and inclusive boundaries, named-filter validation and old-settings compatibility, hostname-based site identity, independent page/file filtering, HEAD-only size checks, excluded metadata requests, unknown lengths, duplicate filename/size selection, redirect scope, request budgets and cancellation during stalled metadata headers.
- Existing transfer, queue, category, columns, browser provenance and cancellation regression tests pass.
- `cargo build --release --locked --bins`: passed without warnings.
- Forty extension automatic-capture mock scenarios and twelve native-protocol/provenance checks pass during packaging.
- Migration audit: all **18,979** references remain valid; **42 partial correspondences**, **18,937 unmapped**, **zero verified complete individual ports**. Sixteen reviewed addresses were added in this revision.
- **38 native dialog templates** now have routes, including original Filters 244 and Filter 245. All recorded original/derived resource hashes pass. Dialog template bytes, fonts, control coordinates and artwork were not edited in this revision. Runtime expansion uses the original lower page controls and original 303-DU host height. Actual rendered behavior still requires desktop acceptance.
- All twelve original column definitions match the original executable evidence. The extension public key and bundled Rust identity match.

## Real browser and installation

RELEASE-ACCEPTANCE.json records a successful isolated Chromium/native-host/download run on the exact 0.5.0 release binaries. Chrome for Testing 156.0.8078.4 / Playwright Chromium 1248 used an Inno Setup 6.7.3 installer with a test-only AppId. The test restored native-host registrations, preserved existing shortcuts and uninstalled its isolated app while retaining its fixture library/downloads.

The installer requested no ID and left browser registrations unchanged. Launching IDX registered the bundled Chrome/Edge identity automatically. The copied extension retained the same public-key identity. Native messaging reported IDX 0.5.0; the popup screenshot was inspected, has no ID copy field/button and fits 401 pixels without horizontal overflow or JavaScript errors. Extension behavior is unchanged from 0.4.0; the version and release instructions were updated.

Two 212,992-byte downloads traversed Chromium, the native host, durable inbox and Rust transfer engine. The category-matched file used its configured folder, while the unmatched file used the default folder. Both matched the fixture bytes; source-page metadata persisted. Actual Chrome download-event permission/cancellation remains mock-tested. Live Edge/Firefox acceptance is pending.

## Grabber behavior and limits

Page and file path rules are separate. Host patterns are case-insensitive; path patterns are case-sensitive. Literal directories match their subtree at a slash boundary; `*` patterns match the corresponding host/path or entire filename. File masks ignore queries. The same hostname counts as the same site across HTTP/HTTPS and ports, as described by the recovered help.

HEAD metadata checks enforce inclusive minimum/maximum sizes and same-name/known-size duplicate filtering. A server without a usable Content-Length is reported as unknown: size limits exclude that file; duplicate-only checking retains it. No GET fallback downloads files to measure them. Metadata has its own 250-request limit, including redirects; the existing crawl limit remains 250 requests, 20,000 links, 4 MiB per HTML page and 32 MiB total HTML. Cancellation interrupts both stages. Selected results retain the first referring page.

Named filters are staged in original manager 244 and editor 245. OK saves the manager changes into JSON settings; closing the manager cancels them. Old settings get compatible defaults, and an explicitly empty filter list persists. Rust uses checked 64-bit sizes and JSON storage rather than the original 32-bit arithmetic/registry IDs. Exact original matching, metadata handling, localization and complete function equivalence have not been established.

## Desktop acceptance

The most recent Computer Use attempt, during 0.4.0, returned `Computer Use was not approved to use IDX - Internet Downloader X` despite the user's consent. Its underlying permission setting is not exposed here. No changed access state was available in this revision, so no repeated prompt or alternate desktop inspection was attempted.

Native Grabber advanced/filter dialogs, Columns, Scheduler, categories and the first-launch guide still require runtime visual comparison with the supplied reference, including high DPI. Static resource integrity and network/controller tests do not prove pixel or full interaction equivalence. Saved projects, offline rewriting, main-domain expansion, authenticated crawling, JavaScript and download-as-discovered remain absent.

## Packages

Use **IDX-Setup-0.5.0.exe** or **IDX-0.5.0-portable.zip** for the desktop application. The release also includes source/reference evidence and current executable copies. Previous releases are retained.

Upload **UPLOAD-TO-CHROME-WEB-STORE-IDX-0.5.0.zip** to the Chrome Developer Dashboard. It is byte-identical to **IDX-Chrome-Extension-0.5.0.zip**, with `manifest.json` at its root and exactly twelve runtime files. Packaging validates ZIP CRCs and the runtime allowlist. No executable, PowerShell script or private key is included. **IDX-Chrome-Listing-Assets-0.5.0.zip** contains listing images/text; older Store Kit, portable and source ZIPs are not extension upload packages.

The publisher must synchronize the Chrome draft public key and rebuild/retest the desktop/extension pair before publication. Public installer/privacy/support URLs remain publisher details. End users never copy IDs. No listing was uploaded or published by this tooling.

`python tools/package_release.py --include-source --publish-root` reproduces the build and packages. `--skip-build` is used only after testing the identical release binaries. CARGO_HOME is `C:\Users\PRS\Desktop\Decomps\.build-cache\cargo`. No Cargo dependency was added for this revision. Static C-runtime linking remains enabled; BINARY-DEPENDENCIES.json records Windows OS-only imports.

## Store upload packaging correction

A Chrome Developer Dashboard upload rejected the original 0.5.0 ZIP because its manifest included `key`. The corrected Store ZIP retains the same extension version and twelve runtime files but omits that development-only field. The installer and portable unpacked extension retain the key to preserve their tested identity. The packager now asserts this separation. Runtime JavaScript and Rust binaries are unchanged. Local ZIP checks pass; Chrome Store acceptance has not yet been observed.
