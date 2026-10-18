# IDX 0.3.0 validation - 17 October 2026

The Cargo/dependency blocker is resolved. This revision builds with the locked dependencies and adds category editing, automatic browser connection and visible branding text updates. Earlier validation is preserved in BUILD-VALIDATION-0.2.0.md and BUILD-VALIDATION-2026-10-15.md.

## Executed checks

- Rust 1.99.1, Windows x64 MSVC, Visual Studio 2022, SDK 10.0.26100.0.
- `cargo fmt --all -- --check`: passed.
- `cargo test --locked`: **64 application + 8 native-host tests passed**. Ten new checks cover categories, controller integration, persisted rename/destinations and browser setup.
- `cargo build --release --locked --bins`: passed without warnings.
- `node browser-extension/tests/automatic-capture.test.cjs`: 40 scenarios passed; `native-protocol.test.cjs`: 12 passed.
- Reference/resource audit: 18,979 references valid, 21 partial correspondences, 18,958 unmapped and zero verified complete individual ports.
- All twelve original column definitions match original binary evidence.
- Templates 100/280 received text-only, equal-size UTF-16 substitutions. Every other byte, including control headers, geometry and fonts, stayed unchanged. Original resources and notices remain preserved.

## Real browser and installation

The isolated acceptance run in RELEASE-ACCEPTANCE.json used Chrome for Testing 156.0.8078.4 / Playwright Chromium 1248 and an Inno Setup 6.7.3 installer with a test-only AppId. It left the user's shortcuts unchanged and restored native-host registry values.

The installer requires no extension ID and leaves browser registrations unchanged. An ordinary IDX launch created the current-user Chrome/Edge connection automatically using the bundled public key. Loading an unpacked copy from another folder retained the same extension identity. Reconnect was idempotent. Real native messaging reported IDX 0.3.0. The popup has no ID field or Copy ID button, fits its 401-pixel viewport, and produced no JavaScript errors.

Two real browser requests traversed the native host, durable inbox and Rust HTTP engine. A matching filename/site used its custom category folder; an unmatched extension used the default folder. Both 212,992-byte files matched the fixture exactly. Parent-page provenance persisted. The test uninstaller preserved the library, completed files and pre-existing shortcuts.

The browser popup was visually inspected. The desktop app ran and transferred files, but native guide/category/Columns/Scheduler controls and exact side-by-side rendering remain unaccepted: Computer Use still returned `Computer Use was not approved to use idx` after the user explicitly allowed inspection. No alternative desktop automation bypass was used.

The browser test invokes the real handoff with the automatic flag. Actual browser download-event permission/cancellation is covered by mocks, not live acceptance. Edge manifests are checked; live Edge and Firefox have not been tested.

## Packaging and reproduction

Version 0.3.0 installer/portable packages contain the tested binaries. The extension ZIP contains Manifest V3 runtime files and a public key, with no executable, PowerShell script, private key or remote code. Store kit screenshots and instructions reflect automatic connection; no store listing has been created or published. The publisher must synchronize the eventual draft's public key and rebuild before publishing the desktop/extension pair.

`python tools/package_release.py` checks formatting/tests, builds both binaries, validates reference integrity and extension identity, checks PE imports and creates the archives. `--include-source --publish-root` also archives sources/references and refreshes the release folder with backups. Use `--skip-build` only for a previously tested identical build. CARGO_HOME here is `C:\Users\PRS\Desktop\Decomps\.build-cache\cargo`.

The C runtime is statically linked. BINARY-DEPENDENCIES.json records OS-only imports. Full functionality and all 18,979 individual ports remain incomplete; MIGRATION-STATUS.md distinguishes partial implementations from verified equivalence.
