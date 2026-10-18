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
    const data = await invoke(api.storage.local, "get", { automatic: false, excludedSites: [] });
    return { automatic: data.automatic === true, excludedSites: normalizeExclusions(data.excludedSites) };
  }

  async function saveSettings(value) {
    const settings = { automatic: value.automatic === true, excludedSites: normalizeExclusions(value.excludedSites) };
    await invoke(api.storage.local, "set", settings);
    return settings;
  }

  async function send(value, automatic = false) {
    const url = validateUrl(value);
    let response;
    try {
      response = firefox
        ? await api.runtime.sendNativeMessage(host, { url, automatic })
        : await new Promise((resolve, reject) => {
          api.runtime.sendNativeMessage(host, { url, automatic }, result => {
            const error = api.runtime.lastError;
            if (error) reject(new Error(error.message)); else resolve(result);
          });
        });
    } catch {
      throw new Error("Cannot connect to IDX. Install IDX and run the browser registration step in BROWSER-INTEGRATION.md.");
    }
    if (!response || response.ok !== true) {
      throw new Error(response && typeof response.error === "string" ? response.error : "IDX did not accept this request.");
    }
    if (automatic && response.queued !== true) {
      throw new Error("Update IDX to enable automatic downloads. The browser download will continue.");
    }
    return response.message || "Download saved in IDX.";
  }

  return { api, firefox, validateUrl, invoke, normalizeExclusions, isExcluded, getSettings, saveSettings, send };
})();
