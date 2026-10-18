# IDX browser integration - 0.7.0

Install **IDX-Setup-0.7.0.exe** and open IDX. The app registers its native host automatically; no extension ID or PowerShell script is needed.

Until a Store version is available, open `chrome://extensions`, enable Developer mode, choose **Load unpacked**, and select the `browser-extension/chromium` folder inside the IDX installation. For an existing unpacked installation pointing at this folder, click **Reload** after updating IDX. Refresh already-open web pages so their content script is updated. Chrome may require confirmation of the new cookies, download and website access permissions.

## Everyday use

- New eligible browser downloads are picked up automatically. IDX pauses the browser transfer while saving the handoff, cancels it after acknowledgement and resumes it on failure. An explicitly saved Off preference is preserved on upgrade.
- A small **Download this video** or **Download this audio** panel appears over detected players. Click it to download, use the arrow to choose a detected file or stream quality, or close it. The panel follows scrolling, resizing and dynamically added players, including embedded frames.
- Clicking the extension's toolbar icon enables/disables integration on the current tab; an X badge indicates disabled integration. There is no toolbar popup requiring a pasted address.
- Right-click a direct link and choose **Download with IDX** for manual handoff.
- The extension's **Options** page provides the connection check, full/mini panel view, automatic-capture and panel switches, site exclusions, and optional manual URL entry.

The browser scripts detect links and draw the panel. The native host, durable inbox, destination selection, transfer engine and library are Rust. Browser media names and Referer values are validated before being forwarded; suggested filenames cannot select a directory or Windows device.

## Current coverage

Direct HTTP/HTTPS media, completed HLS playlists and static single-period DASH are supported. HLS supports master/media playlists, separate default audio, MPEG-TS/fMP4, initialization sections, byte ranges and ordinary AES-128 segment encryption. DASH supports BaseURL inheritance, SegmentTemplate with Number/Time, bounded SegmentTimeline repeats, SegmentList ranges and single-resource SegmentBase. Quality choices appear in the player menu; Best quality is the default. Rust fetches and verifies segments, retains completed segments for resume, and publishes an MP4 after local FFmpeg stream-copy assembly succeeds. The bundled helper is LGPL software with notices in media-tools/. Keep that folder with portable builds.

Live recording, multi-period DASH, HLS discontinuities, DRM/SAMPLE-AES, alternate-audio language selection and site-specific token/signature extraction remain unsupported. AES-encrypted segments are bounded to 256 MiB each; clear media streams directly to disk. A blob URL needs a separately observed media/manifest request. This is not complete original-extension equivalence.

The panel uses original English actions and full/mini choices recovered from the supplied files. CSS recreates the compact bar; it is not a pixel-verified port of the original native overlay. Reference details are in [BROWSER-MEDIA-REFERENCE.md](BROWSER-MEDIA-REFERENCE.md).

## Privacy and permissions

The extension requests cookies, downloads, webRequest and HTTP/HTTPS host access. On a chosen download or stream-quality inspection, matching browser cookies (including HttpOnly and available frame-partitioned cookies) and the browser user-agent go to the local Rust host. It checks domain, host-only, path, secure and expiry rules on every request and redirect. It does not read saved passwords or browsing-history databases.

Cookies pass over native messaging and stdin, never command-line arguments. The brief durable inbox uses Windows current-user DPAPI encryption. Credentials then remain in memory, excluded from the download library and logs. After restarting IDX, send an authenticated link again from the signed-in browser. Expired cookies, site tokens and new unobserved CDN origins may still require another browser visit. Page/source URLs remain in the library. No publisher backend or remote JavaScript is used. Update the Store cookies and privacy disclosures before submitting this version.

The native host permits exact extension origins. Keep `idx-native-host.exe` beside `idx.exe`; portable packages contain both. Reconnect through **IDX Options > General > Extension setup** if needed. Opening a different portable copy makes that installation the current connection.

## Publisher upload

Use **UPLOAD-TO-CHROME-WEB-STORE-IDX-0.7.0.zip**. It contains 15 runtime files with `manifest.json` at its root and no development `key` field. The installer/portable extension retains the development key for a stable unpacked identity. Listing-assets ZIPs are separate and are not installable extensions.

The publisher supplied the Store public key on 18 October 2026. The unpacked manifest and Rust embedded public key now derive the Store identity `bokfgodnpemfnmflphpcoccandblongm`. These public bytes establish the extension identity; they cannot sign releases. No private key is included. Store uploads still omit the manifest `key` field. See [publication validation](PUBLICATION-VALIDATION.md) for the rebuild and its current test limitations. End users never copy an ID.

Project and downloads: https://github.com/itsParassharma/IDX-Internet-Downloader-X

Support: https://github.com/itsParassharma/IDX-Internet-Downloader-X/issues

Privacy: https://github.com/itsParassharma/IDX-Internet-Downloader-X/blob/main/PRIVACY.md

## Validation

Version 0.7.0 passes 132 Rust tests and 97 extension checks. Real Chromium acceptance verifies direct authenticated downloads and five playable stream cases: TS HLS, AES-128 HLS, separate fMP4 tracks, HLS byte ranges and DASH. Output audio/video presence and decoded video frame hashes are checked against synthetic local source media. Rust tests cover pause/resume, missing segments, encrypted inbox data and cross-origin cookie/Authorization stripping. See BUILD-VALIDATION.md and RELEASE-ACCEPTANCE.json. Edge and pixel-level original-overlay acceptance remain pending.
