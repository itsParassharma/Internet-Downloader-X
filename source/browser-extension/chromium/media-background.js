/* SPDX-License-Identifier: MIT */
"use strict";
(() => {
  const api = IDX.api;
  const cache = new Map();
  const requestTabs = new Map();
  const disabled = new Set();
  const mediaOrigins=new Map();
  const ttl = 10 * 60 * 1000;
  const key = (tab, frame) => `media:${tab}:${frame}`;
  let updates = Promise.resolve();
  async function isDisabled(tabId) {
    if (api.storage.session) return (await IDX.invoke(api.storage.session,"get",`disabled:${tabId}`))[`disabled:${tabId}`] === true;
    return disabled.has(tabId);
  }
  globalThis.IDXMediaBackground = {
    metadata(item){const entry=requestTabs.get(item.finalUrl || item.url);return entry && Date.now()-entry.time<60000?{tabId:entry.tabId,frameId:entry.frameId || 0}:{};},
    async isDisabledDownload(item) {
    const entry=requestTabs.get(item.finalUrl || item.url);
    return entry && Date.now()-entry.time<60000 && await isDisabled(entry.tabId);
  }};
  api.action.onClicked.addListener(async tab=>{
    try {
      IDX.validateUrl(tab.url);
      const off=!(await isDisabled(tab.id));
      if(off)disabled.add(tab.id);else disabled.delete(tab.id);
      if(api.storage.session)await IDX.invoke(api.storage.session,"set",{[`disabled:${tab.id}`]:off});
      await api.action.setBadgeBackgroundColor({tabId:tab.id,color:"#606060"});
      await api.action.setBadgeText({tabId:tab.id,text:off?"X":""});
      await api.action.setTitle({tabId:tab.id,title:off?"IDX is disabled on this tab. Click to enable.":"IDX is enabled. Click to disable on this tab."});
      await IDX.invoke(api.tabs,"sendMessage",tab.id,{kind:"idx-media-changed"});
    }catch{ /* browser-owned pages cannot host a panel */ }
  });
  async function read(id) {
    const stored = api.storage.session ? (await IDX.invoke(api.storage.session,"get",id))[id] : cache.get(id);
    return Array.isArray(stored) ? stored.filter(item => Date.now() - item.time < ttl).slice(-64) : [];
  }
  async function remember(id, item) {
    const task = updates.then(async () => {
      const previous = await read(id);
      const old = previous.find(old=>old.url===item.url && old.documentId===item.documentId);
      const items = previous.filter(old => old.url !== item.url);
      items.push({...old,...item,time:Date.now()});
      cache.set(id,items.slice(-64));
      if (api.storage.session) await IDX.invoke(api.storage.session,"set",{[id]:items.slice(-64)});
    });
    updates = task.catch(()=>{});
    await task;
  }
  function inDocument(items, sender) { return items.filter(item => !item.documentId || item.documentId === sender.documentId); }
  api.webRequest.onHeadersReceived.addListener(details => {
    if (details.tabId < 0 || details.incognito || details.statusCode < 200 || details.statusCode >= 300) return;
    requestTabs.delete(details.url);requestTabs.set(details.url,{tabId:details.tabId,frameId:details.frameId,time:Date.now()});
    if(requestTabs.size>256)requestTabs.delete(requestTabs.keys().next().value);
    const header = name => (details.responseHeaders || []).find(h => h.name.toLowerCase() === name)?.value || "";
    const item = IDXMedia.classify(details.url,header("content-type"));
    if (details.type==="media" || item) {
      const id=key(details.tabId,details.frameId),previous=mediaOrigins.get(id)||[];
      if(!previous.includes(details.url))mediaOrigins.set(id,[...previous,details.url].slice(-24));
    }
    if (!item) return;
    const total = /\/(\d+)$/.exec(header("content-range"));
    item.size = Number(total ? total[1] : header("content-length")) || 0;
    item.documentId = details.documentId;
    (async()=>{
      const settings = await IDX.getSettings();
      if (!settings.mediaPanel || await isDisabled(details.tabId) || IDX.isExcluded(details.url,settings.excludedSites) || IDX.isExcluded(details.initiator,settings.excludedSites)) return;
      await remember(key(details.tabId,details.frameId),item);
      await IDX.invoke(api.tabs,"sendMessage",details.tabId,{kind:"idx-media-changed"},{frameId:details.frameId});
    })().catch(()=>{});
  },{urls:["http://*/*","https://*/*"],types:["main_frame","sub_frame","media","xmlhttprequest","other"]},["responseHeaders"]);
  api.runtime.onMessage.addListener((message,sender,respond) => {
    if (!message || !["idx-media-list","idx-media-download","idx-media-info"].includes(message.kind)) return false;
    (async()=>{
      if (sender.id !== api.runtime.id || !sender.tab || sender.tab.incognito) throw new Error("Unavailable on this page.");
      const parent = IDX.validateUrl(sender.url);
      const settings = await IDX.getSettings();
      if (!settings.mediaPanel || await isDisabled(sender.tab.id) || IDX.isExcluded(parent,settings.excludedSites)) return {enabled:false,items:[]};
      const id = key(sender.tab.id,sender.frameId);
      if (message.kind === "idx-media-list") {
        for (const candidate of (Array.isArray(message.sources) ? message.sources : []).slice(0,32)) {
          const item = IDXMedia.classify(candidate.url,candidate.type || "",candidate.kind || "");
          if (item && !IDX.isExcluded(item.url,settings.excludedSites)) await remember(id,{...item,documentId:sender.documentId});
        }
        return {enabled:true,mode:settings.panelMode,items:inDocument(await read(id),sender)};
      }
      const item = inDocument(await read(id),sender).find(item=>item.url===message.url);
      if (!item || !item.supported || IDX.isExcluded(item.url,settings.excludedSites)) throw new Error("This stream is not supported yet. Choose a direct media file.");
      const metadata={tabId:sender.tab.id,frameId:sender.frameId,cookieUrls:mediaOrigins.get(id)||[]};
      if(message.kind==="idx-media-info")return item.stream ? IDX.inspectMedia(item.url,item.stream,parent,metadata) : {ok:true,qualities:[]};
      const filename = IDXMedia.filename(item,typeof message.title === "string" ? message.title.slice(0,160) : "");
      const max_height=Number.isInteger(message.height) && message.height>=0 && message.height<=16384?message.height:0;
      await IDX.send(item.url,true,parent,{filename,referer:parent,...metadata,...(item.stream?{stream:{kind:item.stream,max_height}}:{})});
      return {ok:true,message:"Download saved in IDX."};
    })().then(respond,error=>respond({ok:false,error:error.message}));
    return true;
  });
  async function clearTab(tabId) {
    const prefix = `media:${tabId}:`;
    const names = new Set([...cache.keys()].filter(name=>name.startsWith(prefix)));
    if (api.storage.session) for (const name of Object.keys(await IDX.invoke(api.storage.session,"get",null))) if(name.startsWith(prefix)) names.add(name);
    names.forEach(name=>cache.delete(name));
    for(const name of mediaOrigins.keys())if(name.startsWith(prefix))mediaOrigins.delete(name);
    if (api.storage.session && names.size) await IDX.invoke(api.storage.session,"remove",[...names]);
  }
  api.tabs.onRemoved.addListener(tabId=>{clearTab(tabId).catch(()=>{});disabled.delete(tabId);if(api.storage.session)IDX.invoke(api.storage.session,"remove",`disabled:${tabId}`).catch(()=>{});});
  api.tabs.onUpdated.addListener((tabId,change)=>{if(change.status==="loading") clearTab(tabId).catch(()=>{});});
})();
