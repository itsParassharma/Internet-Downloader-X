# Native UI extraction and coverage

Snapshot: 17 October 2026, release 0.5.0. All 105 original dialog templates and both menu resources are preserved byte for byte in the companion `reverse-engineering/native-ui-resources` folder. The full control inventory is `original-ui-inventory.json`.

**38 native templates are connected to IDX workflows.** Another 62 are extracted and rebranded reference layouts without runtime routes. Five original registration/updater dialogs are deliberately omitted from IDX. A routed layout does not imply that every original control works.

## Connected native surfaces

| Surface | Original dialog IDs |
| --- | --- |
| Main window | 102 |
| Columns customization | 209 |
| Category editor | 204 |
| Browser setup | 381 |
| Download progress and tabs | 103, 363, 364, 365 |
| Download forms, search, confirmations and About | 100, 130, 135, 148, 280, 286, 337, 338, 506 |
| Options pages | 139, 294, 143, 291, 141, 140, 145, 185, 195 |
| Scheduler | 308, 306, 307, 310 |
| Grabber wizard and results | 239, 257, 259, 264, 240, 241, 244, 245 |

The main window uses original RT_DIALOG 102 and menu 129, with the Registration menu removed. The original 3D normal/hover toolbar strips, category/queue icons and Grabber illustration are reused. IDX supplies its own main/window/installer/extension logo. Windows itself draws controls and title bars, so OS theme and DPI affect appearance.

The original resource geometry, control IDs, classes and Tahoma font are retained in derived templates. Runtime Rust code supplies handlers, resizes dynamic panes, adds toolbar/status controls, and adapts captions to supported behavior. The original Options host and other controls created by executable code cannot be recovered as standalone dialog templates; IDX builds those hosts around the extracted pages.

About removes purchase/registration widgets; registration dialog 138 and original updater/component dialogs 144, 182, 190 and 290 are excluded from runtime resources. Optional download-server credentials are separate from product activation. No IDX account, serial key, trial or activation is required.

Options includes all nine extracted pages used by this build; persistent site credentials, dial-up and sound configuration remain disabled. The 15 October baseline supports queue creation/deletion, order, concurrency and weekly times. Release 0.2.0 also connects one-time dates, queue start on application startup, and renaming through the existing name dialog. Runtime acceptance of that revision is pending; see MIGRATION-STATUS.md. Grabber supports bounded recursive HTML scanning, depth and page/file path rules, named type filters, size limits, duplicate filename/size filtering and cancellation. Dialogs 244/245 manage reusable filters. JavaScript execution and saved projects remain unavailable. Native visual acceptance remains pending. See FEATURE-PARITY.md.

## Original Grabber artwork

IMAGE resource 384 (0x180), offset 0x59f790, 22,164 bytes, JPEG 128×260. SHA-256: `53b3b81211c9780b0923b347e5400313f573d8bb143922caae93687dd0ad5de5`.

Retained pseudocode constructors 0x539130, 0x53a790, 0x53d610 and 0x5491b0 load resource 0x180 through 0x5155f0, which uses the IMAGE resource type. Their paint handlers draw it into control 1801 through 0x5156f0. IDX keeps the JPEG bytes unchanged and renders them with Windows GDI+.

## Every original template

| ID | Original title | IDX status |
| --- | --- | --- |
| 100 | About Internet Download Manager | native route with partial behavior |
| 102 | Internet Download Manager 6.43 | native route with partial behavior |
| 103 | Download progress | native route with partial behavior |
| 105 | Tip of the Day | extracted and rebranded reference only |
| 130 | Download File Info | native route with partial behavior |
| 135 | File Properties | native route with partial behavior |
| 138 | Internet Download Manager Registration | omitted registration or original updater |
| 139 | General | native route with partial behavior |
| 140 | Proxy / Socks | native route with partial behavior |
| 141 | Connection | native route with partial behavior |
| 143 | Save to | native route with partial behavior |
| 144 | New version of Internet Download Manager is available | omitted registration or original updater |
| 145 | Sites Logins | native route with partial behavior |
| 146 | Site login | extracted and rebranded reference only |
| 147 | Dialog | extracted and rebranded reference only |
| 148 | Enter new address to download | native route with partial behavior |
| 150 | Shut down | extracted and rebranded reference only |
| 182 | Quick Update | omitted registration or original updater |
| 184 | Dial up | extracted and rebranded reference only |
| 185 | Dial Up / VPN | native route with partial behavior |
| 186 | Dial Up Options | extracted and rebranded reference only |
| 187 | IDM drop target. Drop web-links for downloading here | extracted and rebranded reference only |
| 190 | Download IDM components | omitted registration or original updater |
| 191 | Export download list | extracted and rebranded reference only |
| 193 | Automatically start downloading the following extensions: | extracted and rebranded reference only |
| 194 | Download limits exceeded! | extracted and rebranded reference only |
| 195 | Sounds | native route with partial behavior |
| 196 | Zip preview | extracted and rebranded reference only |
| 204 | IDM categories | native category create/edit route; visual acceptance pending |
| 208 | IDM language | extracted and rebranded reference only |
| 209 | Columns | native route; full build and column tests pass; desktop visual acceptance pending |
| 226 | Max. connections number for a server | extracted and rebranded reference only |
| 228 | Download All Links with IDM | extracted and rebranded reference only |
| 229 | Using special keys | extracted and rebranded reference only |
| 230 | Checking Internet Connection | extracted and rebranded reference only |
| 231 | General | extracted and rebranded reference only |
| 232 | Internet Download Manager Problem Report | extracted and rebranded reference only |
| 236 | IDM tip: Automatic link capture | extracted and rebranded reference only |
| 239 | IDM Site Grabber | native route with partial behavior |
| 240 |  | native route with partial behavior |
| 241 | IDM Site Grabber | native route with partial behavior |
| 242 |  | extracted and rebranded reference only |
| 244 | Filters | native filter manager route; visual acceptance pending |
| 245 | Filter | native name/mask editor route; visual acceptance pending |
| 257 |  | native route with partial behavior |
| 259 |  | native route with partial behavior |
| 262 | Settings of IDM Grabber | extracted and rebranded reference only |
| 264 |  | native route with partial behavior |
| 268 | Template Name | extracted and rebranded reference only |
| 269 | Saving the project of IDM Grabber | extracted and rebranded reference only |
| 270 | Load project | extracted and rebranded reference only |
| 271 | IDM Grabber Statistics | extracted and rebranded reference only |
| 279 | Connecting to the site | extracted and rebranded reference only |
| 280 | Batch download | native route with partial behavior |
| 281 | IDM Site Grabber | extracted and rebranded reference only |
| 284 | Schedule the grabber project | extracted and rebranded reference only |
| 286 | Download complete | native route with partial behavior |
| 290 | IDM update notice | omitted registration or original updater |
| 291 | Downloads | native route with partial behavior |
| 292 | Rebooting computer | extracted and rebranded reference only |
| 294 | File types | native route with partial behavior |
| 296 | Browsers integration | extracted and rebranded reference only |
| 297 | Browser details | extracted and rebranded reference only |
| 298 | Duplicate download link | extracted and rebranded reference only |
| 300 | Internet Download Manager | extracted and rebranded reference only |
| 302 | Cancelling an automatically started download | extracted and rebranded reference only |
| 303 | The list of address exceptions | extracted and rebranded reference only |
| 304 | Add an address to exceptions list | extracted and rebranded reference only |
| 305 | IDM could not take over the download | extracted and rebranded reference only |
| 306 | Schedule | native route with partial behavior |
| 307 | Files in the queue | native route with partial behavior |
| 308 | Scheduler | native route with partial behavior |
| 309 |  | extracted and rebranded reference only |
| 310 | Enter queue name | native route with partial behavior |
| 332 | IDM Queues | extracted and rebranded reference only |
| 334 |  Chrome, Edge, IE  | extracted and rebranded reference only |
| 335 | Firefox and other Mozilla based | extracted and rebranded reference only |
| 336 | Report for IDM developers | extracted and rebranded reference only |
| 337 | Speed Limiter settings | native route with partial behavior |
| 338 | Confirm deletion of downloads | native route with partial behavior |
| 339 | Confirm deletion of downloads | extracted and rebranded reference only |
| 340 |  | extracted and rebranded reference only |
| 341 | Add file type | extracted and rebranded reference only |
| 342 | IDM | extracted and rebranded reference only |
| 344 | IDM | extracted and rebranded reference only |
| 345 |  | extracted and rebranded reference only |
| 348 | Dialog | extracted and rebranded reference only |
| 349 | Enter site name | extracted and rebranded reference only |
| 352 | Exit IDM | extracted and rebranded reference only |
| 363 | Download status | native route with partial behavior |
| 364 | Options on completion | native route with partial behavior |
| 365 | Speed Limiter | native route with partial behavior |
| 367 | Customize download progress dialog | extracted and rebranded reference only |
| 369 | Advanced proxy settings | extracted and rebranded reference only |
| 370 | Download panel exceptions | extracted and rebranded reference only |
| 381 | Google Chrome Integration | IDX browser setup route; automatic connection and extension instructions |
| 487 | Find file | extracted and rebranded reference only |
| 488 | Minimum size | extracted and rebranded reference only |
| 489 | Integration into Microsoft Edge browser | extracted and rebranded reference only |
| 494 | Virus checking | extracted and rebranded reference only |
| 504 |  | extracted and rebranded reference only |
| 505 |  | extracted and rebranded reference only |
| 506 | Find file | native route with partial behavior |
| 30721 | New | extracted and rebranded reference only |
| 30734 |  | extracted and rebranded reference only |

The source now routes original menu command 33002 to dialog 209. Show/Hide, order, width, Reset and OK/Cancel are connected to Rust handlers translated from CEditColumnsDlg. The dialog template is byte-identical to the original (SHA-256 `41dade4edd2ef8a6b71ea2213bab98609d4f877ab0dac1dc4ed878b1a548c5e1`). All 12 original English labels and Reset order/visibility/widths now match the hash-verified executable and recovered routines; see ORIGINAL-COLUMNS.json. Existing saved preferences retain their stable IDX IDs. Native runtime acceptance and localization remain pending.
