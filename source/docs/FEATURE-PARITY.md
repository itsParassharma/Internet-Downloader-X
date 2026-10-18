# IDX feature coverage

Snapshot: 18 October 2026, release 0.7.0. See [MIGRATION-STATUS.md](MIGRATION-STATUS.md) for the audit, release changes and remaining visual-acceptance block. This is an implementation checklist, not a claim of a complete IDM port. “Implemented” means corresponding Rust source was inspected; independent runtime acceptance remains necessary. “Partial” means a simpler workflow exists. Original resource IDs refer to the supplied IDMan.exe, language 1033; full offsets and control captions are in `original-ui-inventory.json`.

## Core download workflow

| Workflow and original evidence | IDX source coverage | Remaining work |
| --- | --- | --- |
| Paste address → choose file/category/folder → start or defer. Dialogs 148, 130; menu 32771 | Implemented: Add download supports direct HTTP(S), destination, category, queue, description, optional website credentials/referer/checksum | Server-derived filename and full original dialog behavior are not established |
| Download file with several connections; resume and report transfer progress. Dialogs 103, 363; string-table Start/Resume action 32790 | Implemented independent segmented HTTP engine, range validation, validators, retries, persistent part manifests, pause/resume, progress and connection list | Not a literal port of original transfer algorithm; original network error/redirect/authentication edge cases remain unmapped |
| FTP access; passive FTP option. Dialog 140 control 1116; translation message 20057 | Absent: URL validation accepts HTTP and HTTPS only | Implement FTP/FTPS deliberately, then test resume/authentication/proxy behavior |
| Edit properties, resume, stop, remove, open file/folder. Dialog 135; menu File | Implemented: primary-row properties/details/open actions and multi-selection resume/pause/removal | Original redownload command 33009 absent; new multi-selection runtime checks pending |
| Duplicate link handling. Dialog 298 controls 1642–1645 | Partial: duplicate destinations receive numbered names; completed files are not overwritten | Original choose-existing/resume/overwrite/remember-choice workflow absent |
| Speed limit and per-file persistence. Dialogs 337, 365 | Implemented: global default plus persisted per-file speed override, shared across each file's segments and adjustable live | Per-file override is available in the progress window; no aggregate application-wide cap |
| Per-server connection limits and timed download quota. Dialogs 141, 226, 309 | Partial: one global connection-count setting, 1–16 per file | Host-specific limits and MB-per-hours quota absent |
| Save categories/extensions/sites, temporary directory, server file timestamp. Dialogs 143, 204 | Partial: editable category names/extensions/site wildcards/folders and remember-last paths; Options stages edits until Apply, Add and browser handoff use shared rules | Native UI acceptance, original rule-precedence edge cases, deletion/reclassification, custom temporary directory and server creation date remain |
| Persistent library and recovery | Implemented independent JSON library, backup/recovery, session-only credentials | Not compatible with original IDM download database or original part files |
| Checksum verification | Implemented optional expected SHA-256 in IDX | IDX feature; this pass did not establish an equivalent original workflow |

## Main interface and menus

The Windows main window now directly instantiates original RT_DIALOG 102 and derived RT_MENU 129. Original 3D toolbar/category pixels, queue icons and Grabber artwork are reused; the main logo is IDX artwork. Native Windows controls use the original Tahoma font and template positions. Dynamic hosts/toolbars and event handlers are new Rust code, and Windows theme/DPI affect rendering. Original resource attribution remains separate from the Rust source license.

The native modules `src/ui/native_main.rs`, `native_forms.rs`, `native_settings.rs`, `native_columns.rs` and `native_progress.rs` route 38 original templates. They include Add URL/File Info, Properties, Batch, Find, Speed Limit, Delete confirmation, completion, About, nine Options pages, Scheduler and Grabber. All 105 original dialogs were extracted; 62 derived layouts remain reference-only, and five registration/updater layouts are excluded. Unsupported controls are disabled, hidden or clearly adapted. See NATIVE-UI-COVERAGE.md for every original dialog's status.

| Original surface | Original command/resource evidence | IDX coverage |
| --- | --- | --- |
| Tasks | Add 32771; batch 34024; batch from clipboard 34036; grabber 33010; drop target 32809; export 32821/32823; import 32816/32818; exit 32772 | Direct/multiline add and bounded numeric/alphabetic range templates, URL text import/export, grabber, exit implemented. Original structured import/export format and drop target absent |
| File | Stop 32774; remove 32775; download now 32776; redownload 33009 | Related transfer/list operations available; redownload not implemented as original workflow |
| Downloads | Pause/stop all 32778/32779; delete completed 32794; find 34040/34041; scheduler 32801; queue start/stop 34026/34028; speed limiter 34033–34035; options 32781 | Core pause, queue start/stop, completed-list cleanup, scheduler/options present. Find/Find next and the dedicated global limiter dialog are implemented |
| View | Categories 32825; sorting 32828/32830/32832/32834/32836/32838/33006/32840/33004/33005/34043; toolbar 33001/34010–34012; tray 34037–34039; columns 33002; dark mode 34042; font 34045/34046; language 32841 | Native light appearance, category views and ascending/descending sorting for displayed columns implemented. Dark-mode switching is disabled in the native Windows UI. Columns customization now included in 0.2.0 through original dialog 209 (visibility/order/width/Reset); all 12 English labels and Reset defaults match binary-derived evidence; runtime acceptance is pending. Toolbar, tray icon, font and translation selection remain absent |
| Search | Dialogs 487, 505, 506; filename/description/site/link/parent-page/referer, case/whole-string options | Native Find dialog 506: filename/description/site/link/parent-page/referer, case/whole-string matching; Find next cycles matches |
| Help | Help 57670; tutorials 32811; scheduler help 34029; grabber help 34022; homepage 32813; support 32784; update 32787; About 32786 | IDX getting-started, browser instructions and About. No legacy help contents or updater |
| Registration | Popup at `0x39FD24`; purchase 32785; registration 32783; dialog 138 | Intentionally absent from IDX |

## Queues and scheduling

| Original workflow | Evidence | IDX coverage / gap |
| --- | --- | --- |
| Create/delete/name a queue | Dialog 308 controls 1675, 1678, 1683 | Create/name/delete implemented. Source now supports rename through the original name dialog, updating all download references; native runtime verification pending |
| Choose simultaneous files and order | Dialog 307 controls 1333/1335; up/down tooltip strings 1229/1230 | Implemented concurrency and up/down ordering |
| Start now, stop, weekly schedule | Dialogs 306/308 | Implemented local weekday/time start and optional pause time while IDX runs. Source revision fixes overnight stops, catches delayed ticks and persists duplicate-event protection; standalone scheduler tests pass |
| One date, repeat interval, synchronization, startup start | Dialog 306 controls 1679, 1584, 1219/1220, 1595, 1729 | One-date scheduling and queue start on application startup included in 0.2.0 using original controls. Repeat intervals and synchronization remain absent; full build and model tests pass; native visual acceptance pending |
| Completion actions | Dialogs 150, 284, 306, 364 | Download notification, open destination folder, and close progress window implemented. Computer shutdown, hang-up and auto-run remain absent |
| Download-complete popup/open-with | Dialog 286 | Original popup 286 implemented with Open, Open with and Open folder |

## Browser, site access and grabber

| Original workflow | Evidence | IDX coverage / gap |
| --- | --- | --- |
| Browser context-menu handoff | Dialog 139 controls 1689/1688; shipped browser extensions | Manifest V3 Chromium extension and native host implement direct-URL/source-page handoff, connection checks and automatic app-launch registration; older Firefox files remain development-only; see `BROWSER-INTEGRATION.md` |
| Automatic browser/system interception, clipboard capture, startup launch | Dialog 139 controls 1751, 1061, 1056; dialogs 296/297 | Default-on browser-download capture with pause/acknowledge/resume-on-failure and site exclusions; real Chromium cancellation and fallback verified. 0.7.0 adds scoped browser cookie/session handoff; system network hooks, clipboard capture and startup launch remain absent |
| In-page media panel, file-type/minimum-size rules, exceptions | Dialogs 340, 370, 488; controls 1728, 1726, 1822 | Partial: floating direct-media panel, full/mini views, file dropdown, Close, frame/dynamic-player detection and MIME-based discovery implemented in 0.6.0. 0.7.0 adds completed HLS/static DASH assembly and quality selection. Live recording, DRM, multi-period DASH, HLS discontinuities, native overlays, minimum-size/file-type rule UI and exact parity remain absent |
| Persistent website credentials | Dialogs 145/146; controls server/path 1337, user 1288, password 1289 | Partial: per-transfer username/password in memory only; no persistent server/path credential manager |
| Proxy/PAC/system proxy/HTTP/HTTPS/FTP/SOCKS credentials | Dialogs 140/369 | Partial: one proxy URL supports request-client proxy configuration; no full original multi-protocol/PAC/bypass rule UI |
| Dial-up/VPN/retry/credential management | Dialogs 184/185/186 | Absent |
| Select links from one page, filter, add selected | Dialog 228; grabber 239 | Native wizard and results list implement bounded recursive HTML scanning, selected-link addition, named include/exclude filename filters and same-name/size duplicate filtering |
| Grabber login and manual login/logout exclusion | Dialog 257 controls 1439, 1441, 1561 | Absent |
| Recursive crawl/same-domain/depth limits/JavaScript | Dialog 264 controls 1435, 1445, 1490/1491, 1493, 1565 | Bounded recursive HTML crawling, internal/external levels, parent-directory restriction and cancellation implemented in 0.4.0. Advanced page path/domain rules and hostname-based site classification added in 0.5.0. Main-domain expansion and JavaScript absent; exact native acceptance pending |
| File type/path/size filters and immediate download | Dialog 240 controls 1515/1516, 1517, 1480/1482, 1483/1486, 1459 | Named file filters (dialogs 244/245), same-host/path restrictions and inclusive size limits implemented in 0.5.0. Bounded HEAD checks verify size and duplicate filename/size; unknown sizes are explicit. Immediate automatic download remains absent |
| Local folder hierarchy and rewritten offline links | Dialog 259 controls 1514, 1468, 1469 | Absent |
| Save/load grabber project/template/statistics | Dialogs 268–271 | Absent |
| Schedule one-time or periodic site synchronization | Dialog 284 controls 1583–1587, 1595 | Absent |

## Additional original surfaces

ZIP preview (196), functional sound playback settings (page 195 is displayed disabled), antivirus program/argument hook (494), toolbar customization, tray/drop target (187), localization (208 and shipped language files), integration troubleshooting (381/489), update dialogs (144/182/290), and problem reports (232/336) remain absent or substantially simplified. Original licensing, activation, trial expiration and registration prompts are intentionally omitted. Real website/proxy authentication is a separate feature and should not be removed merely because its labels include “login”.

## Prioritized additions

1. **Batch ranges implemented:** Add URL now calls `batch::expand_urls`; five tests cover descending/oversized ranges, zero padding, multiple groups and the 1,000-total cap. The native standalone range-generator now uses original dialog 280.
2. **List parity partly implemented:** clickable sorting, Ctrl/Shift multi-selection, Ctrl+A and bulk resume/pause/removal added. New UI verification remains pending; column customization is now included in 0.2.0, while all twelve original English column definitions/defaults are matched to the original binary; localization and exact rendered equivalence remain pending.
3. **Category/queue controls:** queue rename, one-time schedules and queue startup preference are now included in 0.2.0; native acceptance remains pending. Editable categories/directories now have a Rust implementation in 0.3.0; repeat intervals and synchronization remain missing. Existing IDX JSON retains compatible defaults.
4. **Useful grabber parity:** recursive crawling with depth/request bounds and cancellation implemented in 0.4.0; path/size rules and reusable filter editors added in 0.5.0; saved projects remain. Keep JavaScript execution and authenticated browser capture separate features.
5. **Desktop completion:** tray/minimize behavior, explicit startup preference, optional clipboard monitoring, system completion notification and configurable open-with defaults. Verify lifecycle and user-visible state.
6. **Broader protocol/access:** FTP/FTPS, per-server concurrency, website credential store, refined proxy settings and browser-session handoff. These require dedicated authentication/network acceptance cases.

No row is declared “ported” merely because its original pseudocode was exported. A feature is complete only after its IDX workflow and relevant failure/restart cases are verified. See BUILD-VALIDATION.md for release evidence and blocked checks. Earlier Rust tests passed, browser simulations passed, and manual 16 MiB GUI downloads completed with matching SHA-256. The current full MSVC test run passes all 132 tests; the earlier application-control failure is historical. The original-resource inventory itself used static reads only.
