# IDX Chrome Web Store listing - 0.7.0

Use UPLOAD-TO-CHROME-WEB-STORE-IDX-0.7.0.zip as the extension upload. The listing-assets archive is separate. This build does not submit or publish the item.

Name: IDX - Internet Downloader X
Summary: Automatically send downloads to IDX and show a Download this video panel on web players.
Category: Tools
Language: English
Mature content: No
Item support: On
Visibility: Public once the matching desktop build and publisher URLs are ready; keep the draft until then.

## Description

IDX is an open-source alternative to Internet Download Manager, rewritten in Rust for Windows. This extension connects Chrome to IDX, so you can send downloads straight to the app and choose video quality from a small panel above the player. IDX is an independent project, with the full migration still in progress.

IDX connects Chrome to Internet Downloader X, an open-source Windows download manager built in Rust. Install the IDX desktop app, open it once, and enable this extension. No copied extension ID or PowerShell script is needed.

- Automatically send eligible browser downloads to IDX. If the desktop app cannot accept a download, Chrome continues it.
- Use the floating Download this video/audio panel to choose detected files and stream quality.
- Choose full or mini panel view, close individual panels, or exclude sites.
- Click the toolbar icon to enable or disable IDX on the current tab.
- Right-click direct links and choose Download with IDX.
- Use the extension Options page for connection checks and preferences.

Requires IDX 0.7.0 or newer for Windows x64. Automatic capture and media panels are on by default; you can turn them off. Private downloads are excluded.

Direct HTTP/HTTPS media, completed HLS and static single-period DASH are supported, including separate audio/video and ordinary AES-128 HLS. Matching browser login cookies are passed to the local Rust app using an encrypted inbox. Live recording, DRM, HLS discontinuities, multi-period DASH and site-specific token extraction are not supported. A blob URL alone is not a downloadable file.

Media URLs and response metadata are processed locally. Selected downloads, source-page/Referer addresses and scoped login cookies go to the local IDX app; the selected download server receives the requests needed to deliver the file. No analytics, advertising, publisher backend or remote JavaScript is used. See the privacy policy for details.

## Permission explanations

| Permission | Reason |
| --- | --- |
| nativeMessaging | Checks the desktop connection and sends selected downloads to the local Rust host. |
| cookies | Reads matching cookies for selected downloads and stream inspection; preserves domain/path/secure/expiry and browser partition scope. Values are not logged or stored in the download library. |
| downloads | Detects and pauses new downloads; cancels after acknowledgement or resumes on failure. |
| webRequest | Observes URL, MIME and length response metadata to identify direct media files. Does not read response bodies. |
| HTTP/HTTPS host access | Runs player detection in web pages and embedded frames, and observes their media requests. |
| contextMenus | Adds Download with IDX to direct-link/media context menus. |
| activeTab | Reads the active page when the toolbar is clicked or the manual current-page action is used. |
| storage | Stores preferences/exclusions locally and a document-bound media catalog in browser session storage. |

Remote code: none. All scripts are included in the package. Content scripts and host permissions ARE used in this release; replace the older 0.5.0 disclosures.

Homepage: https://github.com/itsParassharma/IDX-Internet-Downloader-X

Installer: https://github.com/itsParassharma/IDX-Internet-Downloader-X/releases/latest

Support: https://github.com/itsParassharma/IDX-Internet-Downloader-X/issues

Privacy: https://github.com/itsParassharma/IDX-Internet-Downloader-X/blob/main/PRIVACY.md

The supplied Store public key now derives bokfgodnpemfnmflphpcoccandblongm in both the unpacked extension and desktop app. No private key is shipped and end users never copy an ID. See docs/PUBLICATION-VALIDATION.md for the rebuilt package and current test limitations.

## Privacy form

Single purpose: Connect Chrome downloads and detected web media to the local IDX Windows download manager, including quality selection and authenticated download handoff.

Data used locally includes authentication information (matching session cookies), web history (download/source URLs and media-request URLs), and website content (detected media elements, addresses and metadata). Do not claim that the extension accesses no user data. It does not read saved passwords, browsing-history databases, payment/health/location data, or personal communications. There is no publisher collection backend, advertising, sale of data or remote JavaScript. Describe local processing and necessary requests to the chosen download servers using the privacy policy.

Use the permission explanations above for each requested permission. Remote code: No. Distribution: Public. Category: Tools. Language: English. Mature content: No.

Use the media-panel and media-menu screenshots for this release; the older popup-only images do not describe the new toolbar behavior.
