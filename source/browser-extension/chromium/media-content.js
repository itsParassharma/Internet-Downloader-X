/* SPDX-License-Identifier: MIT */
/* Player-attached panel. Browser detection stays here; downloads run in Rust. */
"use strict";
(() => {
  if (globalThis.__idxMediaPanel) return;
  globalThis.__idxMediaPanel = true;
  const panels = new Map();
  let scheduled, refreshing = false, pending = false, items = [], mode = "full", enabled = true;
  const css = `
    :host{color-scheme:light}
    *{box-sizing:border-box} .panel{font:12px Tahoma,Arial,sans-serif;color:#101010;pointer-events:auto;max-width:360px}
    .bar{display:flex;width:max-content;max-width:100%;height:25px;border:1px solid #7c9aaa;border-radius:3px;background:linear-gradient(#fdfefe,#dce8ef);box-shadow:0 1px 3px #0005;overflow:hidden}
    button{font:inherit;color:inherit;cursor:pointer;border:0;background:transparent;min-height:23px;padding:2px 6px;white-space:nowrap}
    button:hover{background:linear-gradient(#fffbdc,#f7d880)}button:focus-visible{outline:2px solid #286bb8;outline-offset:-2px}
    button:disabled{color:#666;cursor:default} .main{display:flex;align-items:center;gap:5px}.arrow{font-size:19px;line-height:18px;font-weight:bold;color:#399000;text-shadow:0 1px white}
    .drop,.close{border-left:1px solid #b2c2ce;padding:2px 5px}.close{font-size:15px;color:#555}.menu{margin-top:1px;min-width:220px;max-width:360px;padding:3px;background:#f0f0f0;border:1px solid #888;box-shadow:2px 3px 5px #0004}
    .menu button{display:block;text-align:left;width:100%;white-space:normal;overflow-wrap:anywhere;padding:5px 7px}.menu button:hover{background:#d7e8fb}.menu button:disabled:hover{background:none}
    .status{max-width:320px;white-space:normal;font:11px Tahoma,Arial,sans-serif;background:#fffde4;border:1px solid #9b9b80;padding:5px;margin-top:2px;overflow-wrap:anywhere}.status:empty{display:none}[hidden]{display:none!important}
  `;
  function call(message) {
    return new Promise((resolve,reject)=>chrome.runtime.sendMessage(message,response=>{
      const error=chrome.runtime.lastError;
      if(error)reject(new Error(error.message));else resolve(response);
    }));
  }
  function sources(media) {
    return [...new Set([media.currentSrc,media.src,...Array.from(media.querySelectorAll("source"),s=>s.src)].filter(Boolean))]
      .slice(0,16).map(raw=>{let url=raw;try{const parsed=new URL(raw);parsed.hash="";url=parsed.href;}catch{}return {url,kind:media.localName,type:media.querySelector("source")?.type || ""};});
  }
  function describe(item) {
    const size = item.size > 0 && !item.stream ? ` · ${(item.size / 1048576).toFixed(1)} MB` : "";
    let name;try{name=decodeURIComponent(new URL(item.url).pathname.split("/").pop());}catch{name="";}
    return `${item.format}${size}${item.supported ? "" : " · stream not supported yet"}${name ? ` — ${name.slice(0,90)}` : ""}`;
  }
  function create(media) {
    const host=document.createElement("idx-media-panel");
    host.style.cssText="all:initial!important;position:fixed!important;z-index:2147483647!important;display:block!important;pointer-events:none!important";
    const shadow=host.attachShadow({mode:"open"});
    const style=document.createElement("style");style.textContent=css;
    const panel=document.createElement("div");panel.className="panel";
    const bar=document.createElement("div");bar.className="bar";
    const main=document.createElement("button");main.className="main";
    const arrow=document.createElement("span");arrow.className="arrow";arrow.textContent="⬇";arrow.setAttribute("aria-hidden","true");
    const label=document.createElement("span");main.append(arrow,label);
    const drop=document.createElement("button");drop.className="drop";drop.textContent="▾";drop.title="Choose a file";drop.setAttribute("aria-label","Choose a media file");drop.setAttribute("aria-expanded","false");
    const close=document.createElement("button");close.className="close";close.textContent="×";close.title="Close this panel";close.setAttribute("aria-label","Close this panel");
    const menu=document.createElement("div");menu.className="menu";menu.hidden=true;menu.setAttribute("role","group");menu.setAttribute("aria-label","Detected media files");
    const status=document.createElement("div");status.className="status";status.setAttribute("role","status");
    bar.append(main,drop,close);panel.append(bar,menu,status);shadow.append(style,panel);
    (document.fullscreenElement && document.fullscreenElement.localName!=="video" ? document.fullscreenElement : document.documentElement).append(host);
    const state={host,media,main,label,drop,menu,status,qualities:new Map(),items:[],signature:"",closed:"",busy:false};
    function toggle(open){menu.hidden=!open;drop.setAttribute("aria-expanded",String(open));}
    state.closeMenu=()=>toggle(false);
    async function download(event,item,height=0){
      if(!event.isTrusted || state.busy || !item?.supported)return;
      event.preventDefault();event.stopPropagation();state.busy=true;main.disabled=true;toggle(false);status.textContent="Sending to IDX…";
      try{
        const response=await call({kind:"idx-media-download",url:item.url,title:document.title,height});
        if(!response?.ok)throw new Error(response?.error || "IDX could not accept the download.");
        status.textContent="Download saved in IDX.";
      }catch(error){status.textContent=error.message;}
      finally{state.busy=false;main.disabled=!state.items.some(item=>item.supported);}
    }
    state.download=download;
    main.addEventListener("click",event=>download(event,state.items.find(item=>item.supported)));
    drop.addEventListener("click",event=>{if(!event.isTrusted)return;event.preventDefault();event.stopPropagation();toggle(menu.hidden);if(!menu.hidden)loadQualities(state);});
    close.addEventListener("click",event=>{if(!event.isTrusted)return;event.preventDefault();event.stopPropagation();state.closed=state.signature;host.style.setProperty("display","none","important");});
    panel.addEventListener("keydown",event=>{if(event.key==="Escape"){toggle(false);main.focus();}});
    return state;
  }
  async function loadQualities(state){
    for(const item of state.items.filter(item=>item.stream)) {
      if(state.qualities.has(item.url))continue;
      state.qualities.set(item.url,null);
      try {
        const response=await call({kind:"idx-media-info",url:item.url});
        if(!response?.ok)throw new Error(response?.error || "Could not read stream qualities.");
        state.qualities.set(item.url,response.qualities || []);
      } catch(error){state.qualities.delete(item.url);state.status.textContent=error.message;}
    }
    renderMenu(state);
  }
  function renderMenu(state){
    state.menu.replaceChildren();
    const add=(text,fn,disabled=false)=>{const button=document.createElement("button");button.textContent=text;button.disabled=disabled;button.addEventListener("click",fn);state.menu.append(button);};
    for(const item of state.items){
      add(describe(item)+(item.stream?" · Best quality":""),event=>state.download(event,item),!item.supported);
      for(const quality of state.qualities.get(item.url)||[])add(`${item.format} · ${quality.label}`,event=>state.download(event,item,quality.height));
    }
    if(state.items.filter(item=>item.supported).length>1)add("Download all",async event=>{if(!event.isTrusted)return;for(const item of state.items.filter(item=>item.supported))await state.download(event,item);});
    const line=document.createElement("hr");state.menu.append(line);
    add(mode==="mini"?"Full mode":"Mini mode",async event=>{if(!event.isTrusted)return;await chrome.storage.local.set({panelMode:mode==="mini"?"full":"mini"});state.closeMenu();});
    add("About this panel",event=>{if(!event.isTrusted)return;state.status.textContent='This is the IDX download panel. Customize it in the extension Options page.';state.closeMenu();});
  }
  function place(state){
    const rect=state.media.getBoundingClientRect(),style=getComputedStyle(state.media);
    const visible=enabled && state.items.length && state.closed!==state.signature && rect.width>=40 && rect.height>=20 && rect.bottom>0 && rect.top<innerHeight && rect.right>0 && rect.left<innerWidth && style.visibility!=="hidden" && style.display!=="none";
    state.host.style.setProperty("display",visible?"block":"none","important");
    if(!visible)return;
    state.host.style.setProperty("left",`${Math.max(0,Math.min(innerWidth-(mode==="mini"?85:240),rect.left+4))}px`,"important");
    state.host.style.setProperty("top",`${Math.max(0,Math.min(innerHeight-27,rect.top+4))}px`,"important");
  }
  function render(mediaElements){
    for(const [media,state] of panels)if(!media.isConnected){state.host.remove();panels.delete(media);}
    const active=mediaElements.find(media=>!media.paused) || [...mediaElements].sort((a,b)=>b.clientWidth*b.clientHeight-a.clientWidth*a.clientHeight)[0];
    for(const media of mediaElements){
      const urls=sources(media).map(source=>source.url);
      const matching=items.filter(item=>urls.includes(item.url));
      // Network-only candidates (blob/MSE players, signed URLs) belong to the
      // active/largest player, never to every unrelated player on the page.
      const candidates=matching.length?[...matching,...(media===active && matching.some(item=>item.supported)?items.filter(item=>item.kind===media.localName && !matching.some(match=>match.url===item.url) && !mediaElements.some(other=>other!==media && sources(other).some(source=>source.url===item.url))):[])]:media===active?items.filter(item=>item.kind===media.localName):[];
      if(!candidates.length && !panels.has(media))continue;
      const state=panels.get(media) || create(media);panels.set(media,state);state.items=candidates;
      const signature=candidates.map(item=>item.url).sort().join("\n");
      if(state.signature!==signature){state.status.textContent="";state.closed="";}state.signature=signature;
      state.label.textContent=mode==="mini"?"":`Download this ${media.localName}`;
      state.main.title=`Download this ${media.localName} with IDX`;
      state.main.setAttribute("aria-label",state.main.title);state.main.disabled=state.busy || !candidates.some(item=>item.supported);
      const parent=document.fullscreenElement && document.fullscreenElement.localName!=="video"?document.fullscreenElement:document.documentElement;
      if(state.host.parentElement!==parent)parent.append(state.host);
      renderMenu(state);
      if(!state.menu.hidden && candidates.some(item=>item.stream && !state.qualities.has(item.url)))loadQualities(state);
      place(state);
    }
  }
  async function refresh(){
    if(refreshing){pending=true;return;}refreshing=true;
    try{
      const media=Array.from(document.querySelectorAll("video,audio")).slice(0,32);
      const response=await call({kind:"idx-media-list",sources:media.flatMap(sources).slice(0,32)});
      enabled=response?.enabled===true;mode=response?.mode || "full";items=response?.items || [];
      render(media);
    }catch{enabled=false;panels.forEach(place);}finally{refreshing=false;if(pending){pending=false;schedule();}}
  }
  function schedule(){clearTimeout(scheduled);scheduled=setTimeout(refresh,100);}
  new MutationObserver(schedule).observe(document,{subtree:true,childList:true,attributes:true,attributeFilter:["src"]});
  for(const event of ["loadedmetadata","play","emptied","durationchange"])document.addEventListener(event,schedule,true);
  for(const event of ["scroll","resize"])window.addEventListener(event,()=>panels.forEach(place),{capture:true,passive:true});
  document.addEventListener("fullscreenchange",schedule);
  document.addEventListener("pointerdown",event=>{for(const state of panels.values())if(!event.composedPath().includes(state.host))state.closeMenu();},true);
  chrome.runtime.onMessage.addListener(message=>{if(message?.kind==="idx-media-changed")schedule();});
  chrome.storage.onChanged.addListener((changes,area)=>{if(area==="local" && ["mediaPanel","panelMode","excludedSites"].some(key=>key in changes))schedule();});
  window.addEventListener("pagehide",()=>{panels.forEach(state=>state.host.remove());panels.clear();});
  window.addEventListener("pageshow",schedule);
  schedule();
})();
