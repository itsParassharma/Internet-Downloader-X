const fs=require('node:fs'),vm=require('node:vm'),path=require('node:path'),assert=require('node:assert/strict');
const context={URL};vm.createContext(context);vm.runInContext(fs.readFileSync(path.join(__dirname,'../chromium/media.js'),'utf8'),context);
const {classify,filename}=context.IDXMedia;
let cases=0;
for(const [url,mime,kind,format] of [
  ['https://cdn.example/movie.mp4','','video','MP4'],
  ['https://cdn.example/play?sig=42','video/webm','video','WEBM'],
  ['https://cdn.example/song','audio/mpeg','audio','MP3'],
  ['https://cdn.example/A.M4A?download=1','','audio','M4A']
]){const item=classify(url,mime);assert.equal(item.kind,kind);assert.equal(item.format,format);assert.equal(item.supported,true);cases++;}
for(const url of ['blob:https://site.example/42','data:video/mp4,abc','file:///secret.mp4','https://user:secret@example.com/v.mp4','https://cdn.example/segment.m4s','https://cdn.example/part.ts','https://cdn.example/script.js']){assert.equal(classify(url),null);cases++;}
for(const [url,mime] of [['https://cdn.example/master.m3u8',''],['https://cdn.example/manifest.mpd',''],['https://cdn.example/live','application/vnd.apple.mpegurl']]){assert.equal(classify(url,mime).supported,true);cases++;}
const item=classify('https://cdn.example/videoplayback?id=1','video/mp4');
assert.equal(filename(item,'My film'),'My film.mp4');cases++;
assert.equal(filename(item,'../bad:name'),'.._bad_name.mp4');cases++;
assert.equal(filename(item,'CON'),'_CON.mp4');cases++;
assert.equal(filename(classify('https://cdn.example/real%20name.mp4')),'real name.mp4');cases++;
const longName=filename(item,'日'.repeat(300));assert(Buffer.byteLength(longName)<=220);assert(longName.endsWith('.mp4'));cases++;
assert.equal(classify('https://cdn.example/play?range=0-500','video/mp4'),null);cases++;
console.log(`${cases} media URL, stream and filename checks passed.`);

assert.equal(filename(classify('https://cdn.example/main.m3u8'),'Movie'),'Movie.mp4');
