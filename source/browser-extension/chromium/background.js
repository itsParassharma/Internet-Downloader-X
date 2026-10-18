/* SPDX-License-Identifier: MIT */
"use strict";
if (typeof importScripts === "function") importScripts("native.js", "media.js", "media-background.js");

const menuId = "idx-download";
let downloadsListenerAdded = false;
const handoffs = new Set();

async function report(message, error = false) {
  await IDX.invoke(IDX.api.storage.local, "set", { lastResult: { message, error, time: Date.now() } });
  await IDX.api.action.setBadgeBackgroundColor({ color: error ? "#ad2d2d" : "#19764b" });
  await IDX.api.action.setBadgeText({ text: error ? "!" : "" });
  await IDX.api.action.setTitle({ title: message });
}

async function handleDownload(item) {
  if (!item || item.incognito || item.state !== "in_progress" || item.paused || handoffs.has(item.id)) return;
  if (item.byExtensionId === IDX.api.runtime.id) return;
  if (item.danger && !["safe", "accepted", "allowlistedByPolicy", "deepScannedSafe"].includes(item.danger)) return;
  handoffs.add(item.id);
  let queued = false;
  let pausedByUs = false;
  try {
    const settings = await IDX.getSettings();
    if (!settings.automatic) return;
    if (globalThis.IDXMediaBackground && await IDXMediaBackground.isDisabledDownload(item)) return;
    const rawUrl = item.finalUrl || item.url;
    let url;
    try { url = IDX.validateUrl(rawUrl); } catch { return; }
    if ([item.url, item.finalUrl, item.referrer].some(value => IDX.isExcluded(value, settings.excludedSites))) return;
    const current = await IDX.invoke(IDX.api.downloads, "search", { id: item.id });
    if (!current[0] || current[0].state !== "in_progress" || current[0].paused) return;
    // Hold the browser transfer while the desktop saves the handoff. Resume it
    // on any failure; cancel only after the durable acknowledgement.
    try { await IDX.invoke(IDX.api.downloads, "pause", item.id); pausedByUs = true; } catch { /* already completed, or cannot pause */ }
    // Native success must include a saved-queue acknowledgement before any cancellation.
    const filename = (current[0].filename || item.filename || "").split(/[\\/]/).pop();
    await IDX.send(url, true, item.referrer || "", {
      filename: filename && !filename.endsWith(".crdownload") ? filename : "",
      referer: item.referrer || "",
      ...(globalThis.IDXMediaBackground?.metadata(item) || {})
    });
    queued = true;
    const latest = await IDX.invoke(IDX.api.downloads, "search", { id: item.id });
    if (latest[0] && latest[0].state === "in_progress") {
      try {
        await IDX.invoke(IDX.api.downloads, "cancel", item.id);
        pausedByUs = false;
      } catch {
        await report("Saved in IDX. The browser copy is still downloading.", true);
        return;
      }
    } else {
      await report("Saved in IDX. The browser download already finished or stopped.");
      return;
    }
    await report("Automatic download saved in IDX.");
  } catch {
    await report(queued ? "Saved in IDX. Your browser download has not been stopped." : "IDX could not take over. Your browser download continues.", true).catch(() => {});
  } finally {
    if (pausedByUs) {
      try { await IDX.invoke(IDX.api.downloads, "resume", item.id); } catch { /* finished or removed */ }
    }
    handoffs.delete(item.id);
  }
}

function registerDownloadListener() {
  if (!downloadsListenerAdded && IDX.api.downloads && IDX.api.downloads.onCreated) {
    IDX.api.downloads.onCreated.addListener(handleDownload);
    downloadsListenerAdded = true;
  }
}
registerDownloadListener();
IDX.api.permissions.onAdded.addListener(permissions => {
  if ((permissions.permissions || []).includes("downloads")) registerDownloadListener();
});
IDX.api.permissions.onRemoved.addListener(async permissions => {
  if ((permissions.permissions || []).includes("downloads")) {
    const settings = await IDX.getSettings();
    await IDX.saveSettings({ ...settings, automatic: false });
  }
});

IDX.api.runtime.onInstalled.addListener(async () => {
  if (IDX.firefox) await IDX.api.contextMenus.removeAll();
  else await new Promise(resolve => IDX.api.contextMenus.removeAll(resolve));
  IDX.api.contextMenus.create({
    id: menuId,
    title: "Download with IDX",
    contexts: ["link", "image", "video", "audio"],
    targetUrlPatterns: ["http://*/*", "https://*/*"]
  });
});

IDX.api.contextMenus.onClicked.addListener(async (info,tab) => {
  if (info.menuItemId !== menuId) return;
  try {
    const url=info.linkUrl || info.srcUrl || "",parent=info.frameUrl || info.pageUrl || "";
    await IDX.send(url,false,parent,{referer:parent,tabId:tab?.id,frameId:info.frameId || 0});
    await report("Download saved in IDX.");
  } catch (error) {
    await report(error.message, true);
  }
});
