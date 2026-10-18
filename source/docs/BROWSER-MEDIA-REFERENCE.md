# Browser integration reference review

The supplied `reverse-engineering/extracted-original/IDMGCExt.crx` contains the original Manifest V3 extension, version 6.43.1. Selected files were extracted unchanged into `reverse-engineering/browser-integration-reference/` for local study. Original files retain their copyright and are not relabelled as IDX-authored JavaScript.

Reviewed source:

- `manifest.json`: background service worker, document-start content scripts in all frames, downloads/nativeMessaging/webRequest and host permissions. The original floating integration is not a popup-only link sender.
- `background.js`, `Nb`: attaches download and web-request handlers. `Q.prototype.qb` pauses candidate browser transfers. `kc` cancels accepted transfers or resumes rejected transfers. IDX's acknowledgement is a durable Rust inbox write, not a claim of identical original IPC behavior.
- `background.js`, `Q.prototype.pb`, `R` and `Gb`: toolbar action toggles integration for the current tab and changes the badge/title. IDX 0.6.0 follows that interaction and keeps its settings in the extension Options page.
- `content.js`, `Tb`, `Sb` and neighbouring geometry routines: associates detected URLs with video/audio players, examines source children and player geometry, and reports frame/player information. IDX uses a document-bound URL catalog and a DOM overlay, including dynamic players and embedded frames.
- `document.js`: original page-context fetch/XHR instrumentation and site/player metadata extraction. This has not been transplanted. The 0.7.0 Rust implementation adds HLS/DASH parsing and scoped cookies independently. Site-specific extraction and DRM remain absent.
- `Languages/template.lng`, strings 20850–20862: video/audio, Download this %s, Download actions and Close. Controls 1717/1718 specify Full mode and Mini mode. Controls 1726/1728 and original dialog 340 document media type/panel settings; dialogs 370 and 488 document exceptions and further rules.
- `IDMan-all.c`, `FUN_004ec770` at `0x4ec770`: initializes the original English panel labels and media-size descriptions. This large string-initialization routine has only been consulted; it is not counted as a completed or newly mapped function port.

New executable behavior is written in Rust and browser JavaScript using the supported Chrome APIs. The reference establishes the intended workflow; it does not establish pixel identity, complete streaming support or completion of any of the 18,979 original internal functions. The existing 42 partial mappings and zero verified-complete function ports remain unchanged.

0.7.0 also implements the supplied labels for Download all, About this panel and Full/Mini mode inside the media menu. Stream parsing follows [HLS RFC 8216](https://datatracker.ietf.org/doc/html/rfc8216) and the [DASH-IF timing model](https://dashif.org/Guidelines-TimingModel/). Cookie scope and frame partition selection use the [Chrome cookies API](https://developer.chrome.com/docs/extensions/reference/api/cookies). The original native panel has not yet been verified pixel-for-pixel.
