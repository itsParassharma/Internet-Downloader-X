# Rust migration status — 18 October 2026

The migration is **not complete**. IDX already implements several download-manager workflows independently in Rust, but the 18,979 recovered functions have not been translated or individually verified as equivalent. Version 0.7.0 builds with 132 Rust tests, 97 extension checks and real Chromium acceptance for cookie-authenticated direct downloads, HLS and DASH. Complete side-by-side visual equivalence remains pending.

The user has clarified the acceptance target: the same visible UI and functionality rewritten in Rust, with the entire 18,979-function set implemented or individually mapped to verified equivalent behavior. Reference integrity, partial features and source-only handler ports do not fulfill that target. No function is considered complete solely because it appears to be compiler/framework code.

## Added in release 0.7.0

Completed HLS and static single-period DASH now have independent Rust manifest planning, scoped browser-session requests, verified segment resume and failure handling. The player menu lists stream qualities and includes Download all, Full/Mini mode and About this panel. Ordinary AES-128 HLS, separate audio/video, initialization segments and byte ranges are implemented. A bundled, unmodified LGPL FFmpeg helper stream-copies local tracks into MP4 with network protocols disabled. Cookies use domain/path/secure/expiry checks at each redirect, an encrypted inbox envelope and in-memory download state.

Live recording, DRM/SAMPLE-AES, multi-period DASH, HLS discontinuities, alternate-language selection, token/signature extraction and native-overlay pixel identity remain gaps. The full 18,979-function inventory remains 42 partial mappings, 18,937 unmapped and zero individually verified-complete ports. This feature pass does not change those counts.

## Added in release 0.6.0

The browser extension was previously a popup/manual sender with capture off by default and no player panel. The supplied original CRX and decompiled label initialization confirmed the missing workflows. See [the reference review](BROWSER-MEDIA-REFERENCE.md).

- Capture defaults to on; an explicitly saved Off choice remains off. Chrome downloads pause during native handoff, cancel after durable acknowledgement and resume on failure.
- The toolbar icon toggles integration for the current tab, with an X badge when disabled. Settings and manual entry remain in the Options page.
- A compact player-attached Download this video/audio panel detects direct HTTP(S) sources and media response MIME, follows geometry, handles dynamic players/frames, offers a dropdown, Close and full/mini views.
- Rust validates suggested filenames and source Referer values through the native host and durable inbox. Direct media downloads retain their file type, source page and request Referer.
- HLS/DASH manifests are explicitly unsupported; fragmented/adaptive streams, cookies, DRM, site-specific extraction, detached native overlays and exact visual equivalence remain unimplemented. The panel is a CSS reconstruction, not a verified native rendering port.
- 98 Rust tests and browser checks pass. Live Chromium verified automatic events, browser cancellation after acceptance, browser fallback with an unavailable host, direct media handoff and exact bytes, MIME-only discovery, embedded/dynamic players, live preferences and playlist exclusion.

No new internal function is marked complete. The 42 partial mappings and 18,937 unmapped functions are unchanged; the reviewed original extension JavaScript is outside that internal function inventory.

## Checked inventory

`python tools/audit_migration.py` re-read every indexed pseudocode file, checked its ZIP CRC and nonempty content, reconciled the classification index, and checked original/derived UI resources against their recorded SHA-256 values. All integrity checks passed. See [machine-readable evidence](MIGRATION-AUDIT.json) and [the complete function ledger](FUNCTION-MIGRATION.csv).

| Inventory | Count | Meaning |
| --- | ---: | --- |
| Recovered internal functions | 18,979 | All indexed pseudocode files are available and CRC-valid |
| Exception catch/unwind fragments | 9,293 | Compiler-generated fragments; not 9,293 separate user features |
| Explicit framework/runtime symbol candidates | 1,210 | Classification candidates, not all manually reviewed |
| Named exception-runtime candidates | 9 | Classification candidates |
| Delay-load wrappers / DLL stubs / thunk candidates | 247 | 111 / 27 / 109 respectively |
| Unnamed recovered functions | 7,314 | Need individual behavioral review; may include more runtime code |
| Other named functions | 906 | Need individual behavioral review |
| Reviewed partial Rust correspondences | 42 | Scheduler/name-dialog, Columns, category and Grabber exploration/filter functions; no complete-port claim |
| Individually unmapped functions | 18,937 | No reviewed correspondence recorded yet |
| Verified complete individual function ports | 0 | Function-level equivalence has not been established; working feature implementations do exist |
| Extracted original dialogs | 105 | All retained as references |
| Derived native layouts | 100 | All match recorded hashes |
| Recorded runtime dialog routes | 38 | Routes in the 0.5.0 release, with partial behavior |
| Unrouted derived layouts | 62 | Templates available, behavior not connected |
| Deliberately omitted original product dialogs | 5 | Registration/updater layouts from the existing project scope |

The 18,979 count is a function inventory, not a feature total or a meaningful percentage-complete denominator. The function ledger is conservative: inspected correspondence is different from tested behavioral equivalence.

## What is already in the app

The source and executed tests establish the following coverage. Current release evidence is in [BUILD-VALIDATION.md](BUILD-VALIDATION.md); desktop pixel/control equivalence still needs acceptance.

| Area | Existing implementation | Important remaining work |
| --- | --- | --- |
| Transfers | HTTP/HTTPS segments, validators/ranges, retries, pause/resume, progress, SHA-256 | FTP/FTPS, broader auth/proxy edge cases, server filenames, per-host limits and quotas |
| Download workflow | Add/defer, batch ranges, properties, selected-row actions, completion popup, collision-safe destinations | Original redownload and duplicate-link decisions, structured IDM import/export |
| Main UI | Native dialog 102, menu 129, original toolbar/category pixels, sorting, selection, Find; Columns customization and all 12 original English column definitions/defaults included in 0.2.0 | Configurable toolbars/fonts/language, tray/drop target, full visual acceptance |
| Queues | Create/delete, concurrency, order, start/stop, weekday schedules | Periodic synchronization, queue-specific retry behavior and completion actions |
| Categories | Editable names, extensions, site rules, save folders and remember-last paths; compatible defaults | Native UI acceptance, exact overlapping-rule precedence, delete/reclassification prompts, temporary location, server timestamps |
| Grabber | Native wizard, bounded recursive HTML traversal, internal/external levels, parent-directory restriction, cancellation, selected links, named type filters, page/file path filters, size limits, duplicate filtering and per-link source pages | Main-domain expansion, saved projects, offline links, authentication/JavaScript and native acceptance |
| Browser integration | Direct-link handoff, default-on pickup/fallback, player panel, scoped cookies, completed HLS/static DASH and quality selection | Live/DRM streams, site token extraction, Edge acceptance and exact overlay parity |
| Persistence | JSON library, backup recovery, partial manifests, session-only credentials | Original IDM database/partial-file compatibility, saved site credential manager |
| Desktop/platform | Native progress/completion windows and open/open-with/folder actions | Tray/startup registration, clipboard monitoring, sounds, antivirus hook, dial-up/VPN |

Full control-level gaps and resource IDs remain in [FEATURE-PARITY.md](FEATURE-PARITY.md) and [NATIVE-UI-COVERAGE.md](NATIVE-UI-COVERAGE.md).

## Added in release 0.5.0

The original Grabber Advanced controls now work in pages 264 and 240. The existing page/control geometry, fonts, artwork and template bytes are retained. Expansion reveals the original lower controls, with the 303-DU host height from resource 239; native rendering remains unverified.

- Page exploration includes/excludes paths and domains before following links and redirects. File filters are separate: excluding a page as a downloadable file does not prevent crawling it to find matching files.
- File matching uses complete filename masks, comma-separated `*` patterns and `<start page>`. Query strings do not affect filename matching. The special start-page mask includes the resolved starting page even without a self-link.
- File Advanced controls provide independent path/domain restrictions and inclusive minimum/maximum sizes in Bytes, KB, MB or GB. Values use checked 64-bit arithmetic; invalid or inverted ranges are rejected before a scan.
- Original dialogs 244 and 245 now provide named-filter Add/Edit/Delete, name/mask editing and case-insensitive duplicate-name validation. The manager stages changes until OK; closing it cancels those changes. Accepted filters persist in compatible JSON settings. Existing libraries receive the five default filters; an intentionally empty saved list remains empty. Selection follows edited entries and resets only when its selected filter is deleted.
- The original duplicate checkbox now hides later matches with the same case-insensitive filename and known size, retaining the first discovered copy. Unknown-size files are kept when only duplicate filtering is enabled; size limits exclude them because they cannot be verified. Results report unknown sizes and hidden duplicates.
- Metadata checks use HEAD only, follow at most ten redirect hops and respect file path/site rules before each request. Metadata has a separate 250-request budget including redirects. Page crawling retains its 250-request, 20,000-link and 4-MiB/page / 32-MiB total HTML limits. Closing the wizard cancels a stalled metadata request as well as page reads. No GET fallback downloads a file to measure it.
- Site identity now follows the recovered help: equal hostnames count as the same site across HTTP/HTTPS and ports. Main-domain expansion to subdomains remains absent.

Six filter/compatibility tests and six real HTTP fixture tests cover these rules, boundaries, redirects, metadata-only requests, unknown sizes, duplicate selection, budget exhaustion, parent provenance and cancellation. All 86 application and eight native-host tests pass. Sixteen additional reviewed addresses are recorded as partial correspondences. This is independent Rust behavior connected to recovered handlers; exact original matching, metadata behavior, localization and native UI equivalence remain unverified. Saved projects, offline rewriting, authenticated crawling, JavaScript and download-as-discovered remain absent.

## Added in release 0.4.0

- Original Exploration page **264** now connects whole-site / bounded-level controls **1435/1445**, internal/external depth edits **1406/1472** and parent-directory restriction **1416**. The raw templates, fonts, geometry and artwork are unchanged. Original `grabber.chm/nlevels.html` confirms that level 0 explores only the starting page; downloaded-file links are collected independently of page depth.
- The asynchronous Rust crawler follows HTML navigation links, resolves redirects and HTML base URLs, normalizes fragments, suppresses cycles and keeps the actual source page for each discovered file. Default traversal stays within the final starting URL's origin; external traversal requires nonzero external levels. Parent restrictions and scope checks apply before following child redirects. Known file/media links are listed without downloading their bodies.
- Each scan is bounded to 250 HTTP requests (including redirects), 20,000 distinct links, 4 MiB per page and 32 MiB total HTML. Whole-site mode retains those bounds. Results report limits and skipped failed pages. Closing the wizard drops the active HTTP operation and pending work, including stalled headers/body reads. Stale results cannot overwrite a later scan.
- Ten crawler tests (including the earlier redirect case) plus a controller/persistence test cover local HTTP fixtures, cycles, depths, cross-origin restrictions, redirects, parent directories, budgets, failures, HTML base/entities, stalled cancellation, provenance and restart. The controller never relies on the subsequently edited starting URL for source attribution. All **74 app + 8 native-host** tests pass. A further redirect-boundary test checks that an external redirect back to the base origin cannot bypass its internal level limit. A Windows nonblocking-socket inheritance issue in the new test fixture was found during parallel testing and corrected; three full parallel reruns passed.
- Added an unambiguous **UPLOAD-TO-CHROME-WEB-STORE-IDX-0.4.0.zip** and renamed the separate artwork/text archive to **IDX-Chrome-Listing-Assets-0.4.0.zip**. The extension ZIP has the manifest at its root. Older Store Kit archives contain no manifest and must not be uploaded as the extension.

Five reviewed exploration-dialog correspondences were added to the ledger. They are partial, not complete ports of the original crawl engine. JavaScript execution, authenticated crawling, public-suffix/main-domain expansion, arbitrary HTML MIME/encoding equivalence and saved/offline projects still need work. Advanced filters were subsequently added in 0.5.0. Native desktop inspection was retried against the installed IDX path; Computer Use rejected launch with `Computer Use was not approved to use IDX - Internet Downloader X` despite existing user consent. The settings interface did not resolve this local installation; the underlying permission reason could not be determined, and no bypass was attempted.

## Added in release 0.3.0

- Original category dialog **204** now supports create/edit/rename, extension rules, optional host wildcards, save folders and remember-last paths. Original Save to page **143** exposes its category selector and New/Edit controls. Options edits are staged until Apply/OK; Cancel discards unapplied edits. Renames update download references and the active filter without moving files.
- Main category tree, Add/File Info, batches and browser handoff share the persisted rules. Existing libraries retain the same six categories and destination policy. Explicitly chosen destinations win over automatic category folders. New category names reject reserved/filter/path names and case-insensitive duplicates.
- Matching currently uses the first matching category in saved order, requiring both extension and optional site restriction. Unmatched files use the legacy Other category (retained as fallback after rename). This behavior is tested independently; exact original overlapping-rule precedence remains unverified.
- Browser connection moved out of the installer. Opening IDX registers the bundled extension automatically for the current Windows user; the extension public key gives unpacked copies a stable ID. The first-launch guide and Options > General > Extension setup contain installation instructions and Reconnect, with no copy/paste ID fields. Dialog 381 supplies the existing native host layout. Chrome/Edge activation remains a browser action; no listing has been published.
- Visible About branding and the batch URL example now use IDX/neutral text. Only text bytes changed in templates 100/280; geometry, fonts and control headers were preserved. Original references and resource notices remain retained for provenance.

Eight category/model/controller checks and two browser-setup checks were added to the existing suite. All 64 application and eight native-host tests pass. Source addresses are recorded as partial correspondences, not completed individual ports. The user renewed desktop-inspection consent, but Computer Use still returned `Computer Use was not approved to use idx`; no desktop inspection bypass was attempted.

## Implemented in release 0.2.0

The main window, toolbar, menus and all native template bytes retain their previous layout. New handlers use the original Scheduler pages 306/308 and name dialog 310.

- **One-time date:** original controls 1219/1220 now select a single date. Daily/weekday scheduling remains available through 1221 and the existing day checkboxes.
- **Queue start on application startup:** original checkbox 1729 now persists an opt-in for that queue. Loading resumes its paused/interrupted downloads; completed, failed and verifying entries retain their states. This does not register IDX to launch with Windows.
- **Queue rename:** double-click a queue or press F2 in the Scheduler tree. Original name dialog 310 is prefilled. Validated renaming updates download references and the active queue filter, including completed download records.
- **Overnight stop fix:** a stop time earlier than its start belongs to the next date, even if that next weekday is unchecked. Previously the stop incorrectly required the next weekday to be selected.
- **Delayed timer handling:** while the app is running, delayed ticks apply the latest due start/stop. If both elapsed while sleeping, the queue stays stopped. A fresh app start checks the current minute rather than replaying old missed schedules.
- **Replay protection:** the last automatic transition is persisted. Repeated ticks, restart within the same minute and a backward clock do not replay it. Schedules use local wall-clock time; repeated DST minutes execute at most once.
- **Compatibility:** old JSON queues default to weekly scheduling with startup behavior off. No new Cargo dependency was added.

The original periodic-sync, modem, shutdown and external-program controls remain unimplemented. This revision does not claim original transfer-algorithm equivalence.

### Columns handler translation

Original menu command **33002 — Customize URL List...** now opens original dialog **209 — Columns** through `src/ui/native_columns.rs`. The template is byte-identical to the supplied original, including geometry, font, control IDs and captions (870 bytes, SHA-256 `41dade4edd2ef8a6b71ea2213bab98609d4f877ab0dac1dc4ed878b1a548c5e1`).

The recovered `CEditColumnsDlg` routines supply the initialization, selection/button state, Show/Hide, width-edit, save and Reset behavior. Each reviewed source address is in `FUNCTION-MAPPINGS.json`. Rust now implements checkbox visibility, Move Up/Down, positive decimal width editing with the original `_atol` prefix behavior, Reset, OK/Cancel, stable column IDs and persisted settings. Hidden columns retain their saved widths. Widths use 96-DPI storage and scale for the native list. A timer callback keeps the download controller running while the modal dialog is open.

The supplied installer has now resolved the original definitions. The native list and Columns dialog have all **12 original English labels**. Fresh settings and Reset use the original order, visibility and 96-DPI widths: File Name 150, Date Added 65 (hidden), Q 20, Size 75, Status 65, Time left 75, Transfer rate 95, Last Try Date 65, Description 200, Save To 200 (hidden), Referer 200 (hidden), Parent web page 200 (hidden). Stable IDX subitem IDs preserve existing saved ten-column preferences; the two newly available columns start hidden for those libraries.

`tools/recover_columns.py` reads the verified executable's CString initializer instructions and ASCII payloads, combines them with the recovered Reset routine, and compares the resulting definitions to the actual Rust constants. [ORIGINAL-COLUMNS.json](ORIGINAL-COLUMNS.json) records each original ID, global/string/initializer address and instruction bytes. This check passed. Native runtime behavior after header interactions, localized labels and pixel-level comparison still require acceptance; the dialog is not declared fully equivalent.

Original menu sorting commands 33005 (Referer) and 34043 (Parent Web Page) now operate on distinct fields. `Download.parent_page` defaults to empty for old libraries and persists independently of the HTTP Referer. The single-page Grabber retains the final response URL after redirects and attaches it to selected downloads, even if the editable scan URL subsequently changes. Properties control 1734 displays the recorded source page, and Find's original site/link/parent-page/referer target includes it. The 0.2.0 browser handoff now supplies valid source-page provenance from context-menu and automatic download events, persists it through the durable inbox, and retains it separately from the HTTP Referer. Plain URL imports without source-page metadata still leave it empty.

## Original installer and REA

The user supplied `C:\Users\PRS\Downloads\idman643build15.exe`. Its SHA-256 is `a902e77169a3e4d8a88ee2ccaab11d3d43ab209483625a3602438ed4a22bbfd9`, exactly matching the existing extraction manifest. `tools/extract_reference.py` statically recovered all **192 files** (26,585,498 bytes) into `../reverse-engineering/extracted-original/`, validating every recorded size and SHA-256 before writing. It did not run the installer. The recovered `IDMan.exe` hash is `e3fc655a335e17a6fdd09a9b315291d0c8863180f0d801e5dd7433c9e56afbd8`.

The existing analysis/decompilation Evidence records already identify **REA with Ghidra 12.1.4** as their provider. REA is useful for further native reference/call analysis, but it is not an automatic Rust port. Its current Windows Ghidra route requires a compatible REA package, Ghidra 12.1.x and a full JDK 21 or newer ([official Windows guide](https://github.com/morluto/rea/blob/main/docs/windows-ghidra-p0.md)). Node 26.11.1 and npm 11.20.0 are available here; REA and Ghidra were not found on PATH. The npm metadata request for `rea-agents` failed with `EACCES` connecting to `registry.npmjs.org`. REA was **not installed or registered**; no security or agent configuration was changed. Existing REA-generated evidence remains usable locally.

## Validation and packaging

The previous network/dependency blocker is resolved. Cargo dependencies are cached, build.rs discovers the installed Windows SDK resource compiler, all 86 application and eight native-host tests pass, formatting passes, and both optimized Windows binaries build. The standalone scheduler and column cases are now part of the passing full application suite.

The updated Manifest V3 extension has a connection check, local privacy page and optional automatic capture. IDX registers its native host automatically when opened; the installer has no ID prompt and users need no PowerShell. Forty capture mock scenarios and 12 protocol/provenance checks pass. A real Chromium/native-host/Rust transfer completed with exact bytes and source-page persistence. See BUILD-VALIDATION.md and RELEASE-ACCEPTANCE.json for the executed scope and packaging evidence.

The extension popup has been visually inspected. Native Columns/Scheduler acceptance and side-by-side original IDM comparison remain pending because Computer Use refused access to the IDX window. Grabber/Columns/Scheduler template bytes remain unchanged (About/batch received the documented branding text edits), and all twelve column definitions are checked against the original executable. These checks do not establish exact rendered UI equivalence.

Version 0.5.0 has current executable, installer, portable, source/reference and Chrome extension archives. Older 0.1.0 archives remain available separately. No Chrome Web Store listing has been created or published. The listing assets require publisher URLs; synchronize the draft public key before publishing the desktop/extension pair.

Reproduce the build and packages with `python tools/package_release.py`. Use `cargo test --locked`, `cargo build --release --locked --bins`, `cargo fmt --all -- --check`, `python tools/audit_migration.py --check`, and `python tools/recover_columns.py` for separate checks. Set CARGO_HOME to `C:\Users\PRS\Desktop\Decomps\.build-cache\cargo` to reuse this workspace's populated cache. No security settings were changed.

## Next acceptance and migration work

1. Complete native UI acceptance once Computer Use has app access. Tests/build are now passing. Use isolated `IDX_DATA_DIR` and `IDX_INSTANCE_PORT` environment settings for UI acceptance. Verify Once at/Daily controls, weekday enablement, Apply/reopen persistence, F2/double-click rename, duplicate-name rejection, startup opt-in on two queues, and Columns Show/Hide/reorder/width/Reset/Cancel/persistence. Use the recovered original executable for side-by-side comparison; column definitions are now recovered and source-checked. Confirm the native layout at normal and high DPI before packaging.
2. Accept categories through original dialogs 143/204 against the reference, including overlapping rules, rename/apply/cancel, manual destinations, remember-last paths and high DPI. Continue category deletion/reclassification and original directory-error prompts.
3. Add saved Grabber project workflows through original dialogs 268-271. Recursive scanning, cancellation and advanced filters are implemented; validate their native controls against the reference when app access is available. Continue offline folder/link rewriting and main-domain expansion as separate tested features.
4. Finish duplicate-link/redownload workflows, per-server limits, proxy/site credentials, and desktop integration. Keep browser media capture and FTP/FTPS as separately tested workstreams.
5. Continue function-by-function review in `FUNCTION-MAPPINGS.json`; retain explicit unknowns instead of mechanically translating compiler/runtime fragments or labelling recovered files as finished ports.
