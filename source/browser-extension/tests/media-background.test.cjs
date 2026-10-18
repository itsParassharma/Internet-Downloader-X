const fs=require('node:fs'),vm=require('node:vm'),path=require('node:path'),assert=require('node:assert/strict');
const events={},sent=[],session={},local={};
const event=name=>({addListener:fn=>events[name]=fn});
const store=data=>({get:(keys,cb)=>cb(typeof keys==='string'?{[keys]:data[keys]}:keys===null?{...data}:{...keys,...data}),set:(values,cb)=>{Object.assign(data,values);cb();},remove:(keys,cb)=>{for(const key of [].concat(keys))delete data[key];cb();}});
const chrome={runtime:{id:'idx-test',onMessage:event('message'),sendNativeMessage:(_,message,cb)=>{sent.push(message);cb({ok:true,queued:true});}},
 storage:{local:store(local),session:store(session)},action:{onClicked:event('click'),setBadgeBackgroundColor:async()=>{},setBadgeText:async()=>{},setTitle:async()=>{}},
 webRequest:{onHeadersReceived:event('headers')},tabs:{sendMessage:(...args)=>args.pop()(),onRemoved:event('removed'),onUpdated:event('updated')}};
const context={chrome,URL,Map,Set,Date,Promise};vm.createContext(context);
for(const file of ['native.js','media.js','media-background.js'])vm.runInContext(fs.readFileSync(path.join(__dirname,'../chromium',file),'utf8'),context);
const sender={id:'idx-test',tab:{id:1},frameId:0,documentId:'doc-1',url:'https://site.example/watch'};
const send=(message,from=sender)=>new Promise(resolve=>events.message(message,from,resolve));
const list=sources=>send({kind:'idx-media-list',sources});
(async()=>{
 let cases=0;
 let reply=await list([{url:'https://cdn.example/clip.mp4',kind:'video'}]);assert.equal(reply.enabled,true);assert.equal(reply.items.length,1);cases++;
 reply=await send({kind:'idx-media-download',url:'https://cdn.example/clip.mp4',title:'Movie'});assert.equal(reply.ok,true);assert.equal(sent[0].filename,'clip.mp4');assert.equal(sent[0].referer,sender.url);cases++;
 reply=await send({kind:'idx-media-download',url:'https://cdn.example/unknown.mp4'});assert.equal(reply.ok,false);assert.equal(sent.length,1);cases++;
 reply=await send({kind:'idx-media-list'},{...sender,documentId:'new-doc'});assert.equal(reply.items.length,0);cases++;
 for(const from of [{...sender,id:'another-extension'},{...sender,tab:{id:1,incognito:true}},{...sender,url:'file:///secret'}]){reply=await send({kind:'idx-media-list'},from);assert.equal(reply.ok,false);cases++;}
 await events.click({id:1,url:sender.url});reply=await list([]);assert.equal(reply.enabled,false);cases++;
 events.headers({url:'https://cdn.example/download.zip',tabId:1,frameId:0,statusCode:200,responseHeaders:[]});
 assert.equal(await context.IDXMediaBackground.isDisabledDownload({url:'https://cdn.example/download.zip'}),true);cases++;
 await events.click({id:1,url:sender.url});reply=await list([]);assert.equal(reply.enabled,true);cases++;
 await list([{url:'https://cdn.example/master.m3u8',kind:'video'}]);reply=await send({kind:'idx-media-download',url:'https://cdn.example/master.m3u8'});assert.equal(reply.ok,true);assert.equal(sent.at(-1).stream.kind,'hls');cases++;
 local.excludedSites=['site.example'];reply=await list([]);assert.equal(reply.enabled,false);cases++;
 delete local.excludedSites;events.removed(1);await new Promise(setImmediate);assert(!Object.keys(session).some(key=>key.startsWith('media:1:')));cases++;
 console.log(`${cases} media sender, document, exclusion, toolbar and cleanup checks passed.`);
})().catch(error=>{console.error(error);process.exitCode=1;});
