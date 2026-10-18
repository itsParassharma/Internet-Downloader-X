# IDX privacy

Updated 18 October 2026, for version 0.7.0.

IDX connects Chrome to the download manager on your Windows computer. There is no IDX account, analytics service, advertising service or publisher backend. IDX does not sell your data.

## What the extension uses

Automatic capture reads new download URLs, filenames, source pages and download state. It pauses Chrome's copy while IDX accepts the file, cancels that copy after acknowledgement, and lets Chrome continue if handoff fails. You can turn this off or exclude sites. Incognito use is disabled.

The video/audio panel inspects media elements and their source addresses in pages and embedded frames. The extension observes response URLs, media types and sizes. It does not read response bodies. Detected media addresses are held in browser session storage; they expire from the displayed catalog after ten minutes and are removed when a tab navigates or closes.

Choosing a download sends its address, suggested filename and available source-page/Referer address to the local IDX app. Opening a stream-quality menu sends the manifest address to the local helper for inspection. The Referer may be sent to the download server.

For a selected download or stream inspection, matching login cookies, including HttpOnly and available partitioned cookies, and the browser user-agent are passed to IDX. Cookies keep their domain, path, expiry and secure scope and are used only for matching requests. IDX does not read saved passwords or Chrome's browsing-history database.

## Storage and sharing

Preferences and site exclusions stay on your computer. Accepted downloads and their source-page addresses remain in the desktop library. Cookie values are excluded from the library, extension storage and logs. The short-lived desktop inbox uses Windows current-user encryption; its envelope is removed after acceptance. Credentials then remain in memory for that IDX session. After restarting IDX, send an authenticated link again from its signed-in browser page.

Manifest inspection and file downloads contact the selected download servers. Those servers receive the requests and network information needed to provide the files, including matching authentication cookies when required. IDX does not transmit this information to an IDX-operated service.

## Your choices

Click the toolbar icon to toggle IDX on the current tab. Use the extension's Options page to change automatic capture, media panels, full/mini mode and exclusions. You can restrict site access or remove the extension in Chrome. Removing the extension does not delete your downloaded files or the desktop library.

Information received through Chrome APIs is used only for the download features described here, consistent with the Chrome Web Store User Data Policy and its Limited Use requirements.

For support, [open an issue](https://github.com/itsParassharma/IDX/issues). Issues are public: do not include cookies, passwords or private download URLs.
