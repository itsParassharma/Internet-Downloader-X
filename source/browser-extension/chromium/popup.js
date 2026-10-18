/* SPDX-License-Identifier: MIT */
"use strict";
const input = document.getElementById("download-url");
const status = document.getElementById("status");
const submit = document.getElementById("download");

function show(message, kind = "") {
  status.textContent = message;
  status.className = kind;
}

document.getElementById("current-page").addEventListener("click", async () => {
  try {
    const tabs = IDX.firefox
      ? await IDX.api.tabs.query({ active: true, currentWindow: true })
      : await new Promise(resolve => IDX.api.tabs.query({ active: true, currentWindow: true }, resolve));
    input.value = IDX.validateUrl(tabs[0] && tabs[0].url);
    show("Current page selected. A page address downloads the page itself; choose a direct file link for its media.");
  } catch (error) { show(error.message, "error"); }
});

document.getElementById("download-form").addEventListener("submit", async event => {
  event.preventDefault();
  submit.disabled = true;
  show("Sending to IDX…");
  try {
    show(await IDX.send(input.value), "success");
    await IDX.api.action.setBadgeText({ text: "" });
    await IDX.api.action.setTitle({ title: "Download with IDX" });
  } catch (error) { show(error.message, "error"); }
  finally { submit.disabled = false; }
});

const automatic = document.getElementById("automatic");
const exclusions = document.getElementById("excluded-sites");
const settingsStatus = document.getElementById("settings-status");
const saveSettings = document.getElementById("save-settings");

async function loadSettings() {
  try {
    const settings = await IDX.getSettings();
    automatic.checked = settings.automatic;
    document.getElementById("media-panel").checked = settings.mediaPanel;
    document.getElementById("panel-mode").value = settings.panelMode;
    exclusions.value = settings.excludedSites.join("\n");
    const data = await IDX.invoke(IDX.api.storage.local, "get", { lastResult: null });
    if (data.lastResult) show(data.lastResult.message, data.lastResult.error ? "error" : "success");
  } catch { settingsStatus.textContent = "Could not load settings. Try reopening this panel."; }
}
loadSettings();

const connectionStatus = document.getElementById("connection-status");
const checkButton = document.getElementById("check-connection");
async function checkConnection() {
  checkButton.disabled = true;
  connectionStatus.textContent = "Checking connection…";
  connectionStatus.className = "";
  try {
    const version = await IDX.checkConnection();
    connectionStatus.textContent = `Connected to IDX ${version}`;
    connectionStatus.className = "success";
    document.getElementById("connection-setup").open = false;
  } catch {
    connectionStatus.textContent = "Desktop app not connected";
    connectionStatus.className = "error";
    document.getElementById("connection-setup").open = true;
  } finally { checkButton.disabled = false; }
}
checkButton.addEventListener("click", checkConnection);
checkConnection();

document.getElementById("settings-form").addEventListener("submit", async event => {
  event.preventDefault();
  saveSettings.disabled = true;
  try {
    const excludedSites = IDX.normalizeExclusions(exclusions.value);
    if (automatic.checked) {
      const granted = await IDX.invoke(IDX.api.permissions, "request", { permissions: ["downloads"] });
      if (!granted) {
        automatic.checked = false;
        throw new Error("Browser download access was not enabled. Automatic downloads remain off.");
      }
    }
    const settings = await IDX.saveSettings({ automatic: automatic.checked, excludedSites,
      mediaPanel:document.getElementById("media-panel").checked,panelMode:document.getElementById("panel-mode").value });
    exclusions.value = settings.excludedSites.join("\n");
    settingsStatus.textContent = settings.automatic ? "Saved. New downloads will be sent to IDX." : "Saved. Automatic downloads are off.";
  } catch (error) { settingsStatus.textContent = error.message; }
  finally { saveSettings.disabled = false; }
});
