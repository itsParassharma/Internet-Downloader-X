/* SPDX-License-Identifier: MIT */
/* Shared media metadata rules. Cookie handoff remains in the extension background. */
"use strict";
globalThis.IDXMedia = (() => {
  const formats = {mp4:"video",webm:"video",mkv:"video",mov:"video",m4v:"video",avi:"video",wmv:"video",flv:"video",ogv:"video",mpg:"video",mpeg:"video",mp3:"audio",m4a:"audio",aac:"audio",ogg:"audio",oga:"audio",opus:"audio",wav:"audio",flac:"audio"};
  const mimeExtensions = {"video/mp4":"mp4","video/webm":"webm","video/quicktime":"mov","video/x-matroska":"mkv","video/x-flv":"flv","video/ogg":"ogv","video/mpeg":"mpeg","audio/mpeg":"mp3","audio/mp4":"m4a","audio/aac":"aac","audio/ogg":"ogg","audio/opus":"opus","audio/wav":"wav","audio/x-wav":"wav","audio/flac":"flac","audio/webm":"webm"};
  function classify(raw, mime = "", hint = "") {
    let url;
    try { url = new URL(raw); } catch { return null; }
    if (!["http:","https:"].includes(url.protocol) || url.username || url.password || raw.length > 16384) return null;
    url.hash = "";
    mime = mime.toLowerCase().split(";")[0].trim();
    const ext = url.pathname.split(".").pop().toLowerCase();
    if (url.searchParams.has("bytestart") || /^\d+-\d+$/.test(url.searchParams.get("range") || "")) return null;
    if (["m3u8","mpd","f4m"].includes(ext) || /mpegurl|dash\+xml|f4m/.test(mime)) {
      const format=ext === "mpd" || /dash/.test(mime) ? "DASH" : ext === "f4m" || /f4m/.test(mime) ? "HDS" : "HLS";
      return {url:url.href,kind:"video",format,supported:format!=="HDS",stream:format==="DASH"?"dash":format==="HLS"?"hls":""};
    }
    // A stream fragment is not a complete video, even when its MIME is video/*.
    if (["ts","m4s","cmfv","cmfa"].includes(ext) || mime === "video/mp2t") return null;
    const kind = mime.startsWith("audio/") ? "audio" : mime.startsWith("video/") ? "video" : formats[ext] || hint;
    if (!["audio","video"].includes(kind)) return null;
    const format = formats[ext] ? ext : mimeExtensions[mime] || (kind === "audio" ? "m4a" : "mp4");
    return {url:url.href, kind, format:format.toUpperCase(), supported:true};
  }
  function filename(item, title = "") {
    const extensionFormat=item.stream?"mp4":item.format.toLowerCase();
    let name = "";
    try { name = decodeURIComponent(new URL(item.url).pathname.split("/").pop()); } catch { /* use title */ }
    if (!/\.[a-z0-9]{2,5}$/i.test(name) || !formats[name.split(".").pop().toLowerCase()]) name = `${title || "media"}.${extensionFormat}`;
    name = name.replace(/[<>:"/\\|?*\x00-\x1f\x7f]/g,"_").trim().replace(/[. ]+$/g,"");
    const extension = name.includes(".") ? name.slice(name.lastIndexOf(".")) : `.${extensionFormat}`;
    let stem = Array.from(name.endsWith(extension) ? name.slice(0,-extension.length) : name);
    while (encodeURIComponent(stem.join("")+extension).replace(/%[A-F0-9]{2}/gi,"x").length > 220) stem.pop();
    name = stem.join("") + extension;
    if (/^(con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.|$)/i.test(name)) name = `_${name}`;
    return name || "media.mp4";
  }
  return {classify, filename};
})();
