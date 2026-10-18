/* SPDX-License-Identifier: MIT */
"use strict";

globalThis.IDX = (() => {
  const firefox = typeof browser !== "undefined";
  const api = firefox ? browser : chrome;
  const host = "org.idx.downloader";

  function validateUrl(value) {
    if (typeof value !== "string" || value.length > 16384 || /[\u0000-\u001f\u007f]/.test(value)) {
      throw new Error("Enter a valid HTTP or HTTPS download URL.");
    }
    let url;
    try { url = new URL(value.trim()); } catch { throw new Error("Enter an absolute HTTP or HTTPS URL."); }
    if (!["http:", "https:"].includes(url.protocol) || !url.hostname) {
      throw new Error("Only HTTP and HTTPS download URLs are supported.");
    }
    if (url.username || url.password) throw new Error("URLs with a username or password are not accepted.");
    return url.href;
  }

  function invoke(target, method, ...args) {
    if (firefox) return Promise.resolve(target[method](...args));
    return new Promise((resolve, reject) => {
      target[method](...args, result => {
        const error = api.runtime.lastError;
        if (error) reject(new Error(error.message)); else resolve(result);
      });
    });
  }

  function normalizeExclusions(value) {
    const entries = Array.isArray(value) ? value : String(value || "").split(/[\n,]+/);
    const hosts = entries.map(entry => {
      const text = String(entry).trim().replace(/^\*\./, "");
      if (!text) return null;
      const url = new URL(validateUrl(text.includes("://") ? text : `https://${text}`));
      return url.hostname.toLowerCase().replace(/\.$/, "");
    }).filter(Boolean);
    return [...new Set(hosts)];
  }

  function isExcluded(value, hosts) {
    if (!value) return false;
    try {
      const hostname = new URL(value).hostname.toLowerCase().replace(/\.$/, "");
      return hosts.some(host => hostname === host || hostname.endsWith(`.${host}`));
    } catch { return false; }
  }

  async function getSettings() {
    const data = await invoke(api.storage.local, "get", { automatic: true, mediaPanel: true, panelMode: "full", excludedSites: [] });
    return { automatic: data.automatic === true, mediaPanel: data.mediaPanel !== false,
      panelMode: data.panelMode === "mini" ? "mini" : "full", excludedSites: normalizeExclusions(data.excludedSites) };
  }

  async function saveSettings(value) {
    const settings = { automatic: value.automatic === true, mediaPanel: value.mediaPanel !== false,
      panelMode: value.panelMode === "mini" ? "mini" : "full", excludedSites: normalizeExclusions(value.excludedSites) };
    await invoke(api.storage.local, "set", settings);
    return settings;
  }

  async function request(message) {
    let response;
    try {
      response = firefox
        ? await api.runtime.sendNativeMessage(host, message)
        : await new Promise((resolve, reject) => {
          api.runtime.sendNativeMessage(host, message, result => {
            const error = api.runtime.lastError;
            if (error) reject(new Error(error.message)); else resolve(result);
          });
        });
    } catch {
      throw new Error("IDX is not connected. Open the IDX Windows app once to connect automatically. If needed, choose Reconnect browser in Options > General > Extension setup.");
    }
    if (!response || response.ok !== true) {
      throw new Error(response && typeof response.error === "string" ? response.error : "IDX did not accept this request.");
    }
    return response;
  }

  async function checkConnection() {
    const response = await request({ action: "ping" });
    if (response.protocol !== 2 || typeof response.version !== "string") {
      throw new Error("Update the IDX desktop app to version 0.2.0 or newer, then check again.");
    }
    return response.version;
  }

  async function browserContext(url, metadata = {}) {
    if (!api.cookies) return null;
    const urls = [...new Set([url, ...(metadata.cookieUrls || []).slice(0, 24)])].map(validateUrl);
    let storeId;
    if (Number.isInteger(metadata.tabId) && api.cookies.getAllCookieStores) {
      const stores = await invoke(api.cookies, "getAllCookieStores");
      storeId = stores.find(store => store.tabIds.includes(metadata.tabId))?.id;
      if (!storeId) throw new Error("The browser session is no longer available. Refresh the page and try again.");
    }
    let partitionKey;
    if (Number.isInteger(metadata.tabId) && api.cookies.getPartitionKey) {
      try { ({partitionKey} = await invoke(api.cookies, "getPartitionKey", {tabId:metadata.tabId, frameId:metadata.frameId || 0})); } catch { /* older browsers provide unpartitioned cookies only */ }
    }
    const collected = new Map();
    for (const address of urls) {
      const parsed = new URL(address), host = parsed.hostname.toLowerCase();
      const queries = [{url:address}, {domain:host}];
      if (partitionKey?.topLevelSite) queries.push({url:address,partitionKey},{domain:host,partitionKey});
      for (const query of queries) {
        const cookies = await invoke(api.cookies, "getAll", {...query,...(storeId ? {storeId} : {})});
        for (const cookie of cookies) {
          const domain=cookie.domain.replace(/^\./, "").toLowerCase();
          if (!(host===domain || (!cookie.hostOnly && host.endsWith(`.${domain}`))) || (cookie.secure && parsed.protocol!=="https:") || (cookie.expirationDate && cookie.expirationDate<=Date.now()/1000)) continue;
          // Never mix another top-level site's partition into this handoff.
          if (cookie.partitionKey && (!partitionKey || cookie.partitionKey.topLevelSite!==partitionKey.topLevelSite || cookie.partitionKey.hasCrossSiteAncestor!==partitionKey.hasCrossSiteAncestor)) continue;
          collected.set(`${domain}|${cookie.path}|${cookie.name}`,{name:cookie.name,value:cookie.value,domain,path:cookie.path,secure:cookie.secure,host_only:cookie.hostOnly,expires:cookie.expirationDate || null});
        }
      }
    }
    const context={cookies:[...collected.values()],user_agent:globalThis.navigator?.userAgent || ""};
    if(context.cookies.length>256 || JSON.stringify(context).length>180000)throw new Error("This browser session is too large to hand off safely.");
    return context;
  }
  async function inspectMedia(value, kind, parentPage, metadata={}) {
    const url=validateUrl(value), browser=await browserContext(url,metadata);
    return request({action:"media_info",url,kind,referer:validateUrl(parentPage),...(browser?{browser}:{})});
  }

  async function send(value, automatic = false, parentPage = "", metadata = {}) {
    const url = validateUrl(value);
    const message = { url, automatic };
    // Page provenance is optional; do not send browser-internal URLs or credentials.
    if (parentPage) {
      try { message.parent_page = validateUrl(parentPage); } catch { /* unknown source */ }
    }
    if (metadata.filename) message.filename = metadata.filename;
    if (metadata.referer) message.referer = validateUrl(metadata.referer);
    if (metadata.stream) message.stream=metadata.stream;
    const browser=await browserContext(url,metadata);if(browser)message.browser=browser;
    const response = await request(message);
    if (response.queued !== true) {
      throw new Error(automatic
        ? "IDX did not confirm saving the request. The browser download will continue."
        : "IDX did not confirm saving the request. Update the IDX desktop app and try again.");
    }
    return response.message || "Download saved in IDX.";
  }

  return { api, firefox, validateUrl, invoke, normalizeExclusions, isExcluded, getSettings, saveSettings, send, checkConnection, browserContext, inspectMedia };
})();
