# IDX - Internet Downloader X

IDX is an open-source alternative to Internet Download Manager, rewritten in Rust for Windows. Download files with several connections, pause and resume transfers, organise them into queues, and pick video quality straight from Chrome. No account, activation key or trial countdown. IDX is an independent project; the full migration is still in progress.

**Release 0.7.0, 18 October:** The floating media panel now downloads completed HLS and static DASH streams, with quality selection, separate audio/video assembly, AES-128 HLS and byte ranges. Browser login cookies are scoped to each request and passed through an encrypted Windows inbox. Rust handles network traffic and manifests; a bundled LGPL FFmpeg helper assembles local tracks. Automatic pickup and the compact panel remain available. Exact original-overlay and site-by-site parity remain unfinished. See [validation](docs/BUILD-VALIDATION.md) and [browser instructions](docs/BROWSER-INTEGRATION.md).

## Install or run

- **Installer:** run `IDX-Setup-0.7.0.exe`. It defaults to `%LOCALAPPDATA%\Programs\IDX` and installs for the current Windows user.
- **Portable:** extract `IDX-0.7.0-portable.zip`, then open `idx.exe`. Keep `idx-native-host.exe` beside it if using the optional browser extension. The portable executable still saves its library in the user-data location described below.
- **Browser handoff:** open IDX once and load/enable the supplied extension. The app configures the connection automatically. The first-launch guide is also available in **Options > General > Extension setup**; see [browser instructions](docs/BROWSER-INTEGRATION.md).

## Chrome Web Store upload

Upload **UPLOAD-TO-CHROME-WEB-STORE-IDX-0.7.0.zip** directly. Its root contains `manifest.json`; the Store upload manifest omits the development-only `key` field. The installed/unpacked extension keeps its stable public key. The separate **IDX-Chrome-Listing-Assets-0.7.0.zip** contains screenshots and listing copy. Older Store Kit, portable and source archives are not extension upload packages. See [publisher details](docs/BROWSER-INTEGRATION.md) for synchronizing the draft public key before publication.

## Download a file

1. Select **Add URL** and paste a direct HTTP or HTTPS file address. Use **Add batch download** for URL ranges or the clipboard-batch menu for multiple addresses.
2. Choose a destination, filename and category in **Download File Info**. **More...** contains queue, referer and SHA-256 options. **Download later** keeps it paused until you resume it or start its queue.
3. Choose **Start Download**, or start a queue. Double-click a row to open its detailed progress window.
4. **Stop/Pause** pauses the transfer. **Resume** continues safely when the server supports suitable byte ranges and a matching validator; otherwise the transfer may need to restart.
5. Use **Scheduler** for queue concurrency, file order and weekly or one-time start/stop times. IDX must be running at the scheduled time.

Advanced fields accept website credentials, a referer and an expected SHA-256. Website credentials are optional credentials for the download server, not an IDX account. TLS certificate verification remains enabled.

Use **Options > Save to > New/Edit** to customize categories. Rules check filename extensions and optional site patterns; new downloads use the selected category folder. Category renames preserve existing file paths.

Use **Grabber** to find HTTP(S) links in HTML pages. Level 0 scans the starting page; higher levels follow page links. Choose internal/external levels or Explore the whole site (still capped at 250 requests). Close cancels an active scan. In **Exploration > Advanced**, include or exclude page paths/domains. In **Filters > Advanced**, choose file paths and minimum/maximum sizes. **Add Filter** opens the reusable name/mask list; use patterns such as `image*.jpg,*.png`. The duplicate option keeps the first file with the same name and known size. Files with unknown size are excluded when a size limit is enabled. After scanning, check the results and add selected links. Use **Import URL list** and **Export download URLs** for plain-text lists.

## Included capabilities

- Segmented HTTP/HTTPS transfers, range/validator checks, retries and pause/resume.
- Per-file progress, transfer rate, remaining time and connection progress.
- Multiple named queues, rename, concurrency, queue ordering, weekly/one-time schedules and opt-in queue startup.
- Editable file categories, extension/site rules and per-category save folders, filename collision handling and Find/Find next.
- Proxy URL, custom user agent, per-download speed limit and session-only website credentials.
- Optional SHA-256 verification, persistent download library and backup recovery.
- Original 3D toolbar and category icons, a distinct IDX logo, native Windows controls and bounded recursive HTML link grabber.
- Browser context-menu handoff, automatic pickup with site exclusions, and floating panels for direct video/audio files.
- Original native main-window template and menu, plus extracted Add URL, File Info, Properties, Batch, Find, Options, Scheduler, Grabber and completion dialogs. The separate progress window includes the connection map/table, Speed Limiter and completion options.

Release 0.7.0 connects 38 extracted native templates. All 105 original templates are included as reference; this does not mean all original functionality has been implemented. See [native UI coverage](docs/NATIVE-UI-COVERAGE.md). Unsupported commands are disabled or hidden.

Add URL and batch entry accept inclusive numeric/alphabetic templates such as `file[001-025].zip` and `[a-z]`, capped at 1,000 expanded URLs. Multiple groups form combinations, for example `/[a-b]/[01-02]`.

Click a download-list heading to sort; click again to reverse it. Ctrl-click toggles a row, Shift-click selects a displayed range, and Ctrl+A selects the filtered list. Resume, Pause and Remove apply to selected entries; properties and details use the primary row. Bulk removal preserves downloaded files and partial data.

## Current limits

FTP/FTPS, live stream recording, DRM processing, multi-period DASH, HLS discontinuities, offline site mirroring, saved grabber projects, tray/startup integration and legacy database import are not implemented in this release snapshot. The browser extension sends direct addresses you select, or eligible new browser downloads when automatic capture is enabled; it cannot turn a protected page or `blob:` URL into a downloadable media file. The Grabber does not execute JavaScript; scans are capped at 250 page requests, 20,000 links, 4 MiB per page and 32 MiB total HTML. Size/duplicate checks allow up to 250 additional metadata requests; file bodies are not downloaded during those checks.

The original application contains substantially more configuration and platform integration. The feature matrix lists missing workflows rather than implying that decompilation automatically provided them.

## Where data is saved

- Default downloads: the Windows Downloads folder, inside `IDX`.
- Library/settings: `%LOCALAPPDATA%\IDX\library.json` with a backup beside it.
- Incomplete transfer data: engine-managed sidecar directories beside the intended destination.
- Browser handoff requests: `%LOCALAPPDATA%\IDX\inbox`.

Removing an entry from the download list does not delete its completed file or partial data. Website passwords are retained only for the running session. Proxy URLs containing credentials are not persisted. An alternate library directory can be supplied with `idx.exe --data-dir "D:\IDX-data"`; browser handoff inbox location remains the default. For isolated development runs, set absolute IDX_DATA_DIR and a separate nonzero IDX_INSTANCE_PORT; native-host child processes inherit both.

The command `idx.exe --add "https://example.com/file.zip"` sends a direct download address to IDX. The native host uses the short-lived `--enqueue URL [--automatic]` command and waits for a saved-request acknowledgement before the extension cancels a browser download.

## Build from source

Requirements: Windows x64, Rust 1.88 or newer, a matching Windows linker/resource compiler, and optional Inno Setup 6.5+ for the installer. The MSVC toolchain requires its build tools and Windows SDK (`link.exe` and `rc.exe`). The GNU toolchain requires MinGW-compatible tools, including `windres` or `llvm-windres`. Keep `Cargo.lock` for repeatable dependency versions.

From this `source` directory in a configured developer terminal:

```powershell
cargo test --locked
cargo build --release --locked --bins
```

The default build puts `idx.exe` and `idx-native-host.exe` in `target\release`. Explicit Rust targets use `target\TARGET\release`.

To test, build and package the app and extension using Python 3.11+, Node.js and Inno Setup:

```text
python tools/package_release.py
```

Use `--skip-installer` without Inno Setup; `--skip-build` reuses an already tested release. `--include-source` archives source/references. `--publish-root` backs up and refreshes the parent release folder. See [packaging details](docs/BUILD-VALIDATION.md). MSVC resource discovery now finds installed Windows SDK versions automatically; IDX_RC/RC and IDX_ISCC can select explicit compilers. The optional build.ps1 developer wrapper uses the same packager.

## Source map and validation

| File or folder | Purpose |
| --- | --- |
| `src/engine.rs`, `src/engine_tests.rs` | Transfer engine and network-behavior tests |
| `src/model.rs` | Persistent settings/download library, categories and file naming |
| `src/ui.rs` | Shared download controller, queues, scheduling and transfer events |
| `src/ui/native_main.rs`, `native_forms.rs`, `native_settings.rs` | Windows main window and extracted dialogs |
| `src/ui/native_progress.rs`, `native_resources.rs` | Separate transfer window and embedded resource templates |
| `src/ui/dialogs.rs` | Retained non-Windows immediate-mode UI |
| `src/grabber.rs` | Single-page link extraction |
| `src/batch.rs` | Bounded numeric/alphabetic URL expansion |
| `src/bin/idx-native-host.rs` | Native browser messaging bridge |
| `browser-extension/` | Current Chromium extension/store kit and older Firefox development files |
| `installer/idx.iss`, `tools/package_release.py` | Installer and package build |
| `docs/` | Browser setup, feature matrix and static-analysis evidence |

Automated checks cover segmented downloads, retry/resume, content changes, missing range support, unknown/empty lengths, long Windows paths, no-overwrite races, library recovery, verification, URL batches, sorting/selection and the native protocol. The extension has 40 simulated capture checks, 12 protocol/provenance checks and a passing real Chromium/native-host transfer test. See [build validation](docs/BUILD-VALIDATION.md) for release results and unverified areas.

The full reference archive was checked: all **18,979** indexed pseudocode files are present, non-empty and pass ZIP CRC validation. Original normal/hover 3D toolbar pixels match the supplied BMPs exactly; the new main logo is different. [Verification details](docs/VERIFICATION.json) expressly record that the complete original function set has **not** been ported to Rust.

## License

IDX's original Rust source is released under the [MIT license](LICENSE). Selected supplied icons and other UI resources are included at the user's request under the rights the user asserted; they retain their original copyright and are **not automatically covered by IDX's MIT grant**. See `RESOURCE-NOTICE.txt` in the installed/portable package or `assets/RESOURCE-NOTICE.txt` in the source. The separate recovered original pseudocode is also outside the IDX MIT grant.

Third-party dependencies and embedded fonts retain their own licenses. [Third-party notices](THIRD-PARTY-NOTICES.txt) identify all locked dependencies and the texts in `third-party-licenses/`, including Rust and Windows runtime notices. IDX is an independent implementation, not an official IDM release.
