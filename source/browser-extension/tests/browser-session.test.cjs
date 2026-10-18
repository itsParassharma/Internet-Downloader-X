const fs=require('node:fs'),vm=require('node:vm'),path=require('node:path'),assert=require('node:assert/strict');
const queries=[],messages=[];
const cookies=[
 {name:'session',value:'test-secret',domain:'media.example',path:'/private',secure:true,hostOnly:true,httpOnly:true,storeId:'test'},
 {name:'foreign',value:'never-forward',domain:'other.example',path:'/',secure:true,hostOnly:true},
 {name:'expired',value:'old',domain:'media.example',path:'/',secure:true,hostOnly:true,expirationDate:1},
 {name:'partition',value:'correct-partition',domain:'media.example',path:'/',secure:true,hostOnly:true,partitionKey:{topLevelSite:'https://site.example',hasCrossSiteAncestor:true}},
 {name:'partition',value:'wrong-partition',domain:'media.example',path:'/',secure:true,hostOnly:true,partitionKey:{topLevelSite:'https://other.example',hasCrossSiteAncestor:true}}
];
const chrome={cookies:{getAll(q,cb){queries.push(q);cb(cookies);},getAllCookieStores(cb){cb([{id:'test',tabIds:[7]}]);},getPartitionKey(q,cb){assert.equal(q.tabId,7);cb({partitionKey:{topLevelSite:'https://site.example',hasCrossSiteAncestor:true}});}},runtime:{sendNativeMessage(host,message,cb){messages.push(message);cb(message.action==='media_info'?{ok:true,qualities:[{height:720,label:'720p'}]}:{ok:true,queued:true});}}};
const context={chrome,URL,Map,Set,Date,navigator:{userAgent:'Fixture Browser'}};vm.createContext(context);vm.runInContext(fs.readFileSync(path.join(__dirname,'../chromium/native.js'),'utf8'),context);
(async()=>{
 const session=await context.IDX.browserContext('https://media.example/private/master.m3u8',{tabId:7});
 assert.deepEqual(JSON.parse(JSON.stringify(session.cookies.map(c=>c.name))),['session','partition']);
 assert(session.cookies.some(c=>c.value==='correct-partition'));assert.equal(session.user_agent,'Fixture Browser');
 assert(queries.every(q=>(q.url||q.domain) && q.storeId==='test'));
 await context.IDX.send('https://media.example/private/master.m3u8',true,'https://site.example/watch',{tabId:7,stream:{kind:'hls',max_height:720}});
 assert.equal(messages[0].browser.cookies[0].value,'test-secret');assert.equal(messages[0].stream.max_height,720);
 const reply=await context.IDX.inspectMedia('https://media.example/private/master.m3u8','hls','https://site.example/watch',{tabId:7});assert.equal(reply.qualities[0].height,720);
 assert.equal(messages[1].action,'media_info');
 await assert.rejects(()=>context.IDX.browserContext('https://media.example/private/file',{tabId:99}));
 const insecure=await context.IDX.browserContext('http://media.example/file',{});assert.equal(insecure.cookies.length,0);
 console.log('8 browser cookie scope, partition, store, metadata and quality checks passed.');
})().catch(error=>{console.error(error);process.exitCode=1;});
