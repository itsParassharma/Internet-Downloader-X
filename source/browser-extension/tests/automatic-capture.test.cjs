const fs = require('node:fs');
const vm = require('node:vm');
const assert = require('node:assert/strict');
const root = require('node:path').join(__dirname, '..', 'chromium') + require('node:path').sep;
const nativeCode = fs.readFileSync(root + 'native.js', 'utf8');
const backgroundCode = fs.readFileSync(root + 'background.js', 'utf8');
function environment(firefox, options = {}) {
  const events = {}, order = [], sent = [], cancelled = [];
  const storage = { automatic: options.automatic !== false, excludedSites: options.exclusions || [] };
  const current = { id: 7, state: 'in_progress', paused: false };
  const event = name => ({ addListener(fn) { events[name] = fn; } });
  const api = { runtime: { id: 'idx-test', onInstalled: event('installed') },
    action: { setBadgeBackgroundColor: async()=>{}, setBadgeText:async()=>{}, setTitle:async()=>{} },
    contextMenus: { removeAll: callback => { if(callback)callback(); else return Promise.resolve(); }, create:()=>{}, onClicked:event('menu') },
    permissions: { onAdded:event('permissionAdded'), onRemoved:event('permissionRemoved') },
    downloads: { onCreated:event('download') }, storage: { local: {} } };
  function method(target, name, fn) {
    target[name] = (...args) => {
      if(firefox) return Promise.resolve().then(()=>fn(...args));
      const cb=args.pop(); Promise.resolve().then(()=>fn(...args)).then(cb, error => { api.runtime.lastError={message:error.message}; cb(); delete api.runtime.lastError; });
    };
  }
  method(api.storage.local,'get',defaults=>({...defaults,...storage}));
  method(api.storage.local,'set',data=>Object.assign(storage,data));
  method(api.downloads,'search',()=>[{...current}]);
  method(api.downloads,'pause',()=> { if(options.pauseFails)throw new Error('cannot pause');order.push('pause');current.paused=true; });
  method(api.downloads,'resume',()=> { order.push('resume');current.paused=false; });
  method(api.downloads,'cancel',id=> { if(options.cancelFails)throw new Error('cancel failed'); order.push('cancel'); cancelled.push(id); current.state='interrupted'; });
  method(api.permissions,'request',()=>true);
  method(api.runtime,'sendNativeMessage',async(name,message)=>{
    order.push('send'); sent.push({name,message});
    if(options.gate)await options.gate;
    if(options.nativeFails)throw new Error('host unavailable');
    if(options.completeDuringHandoff)current.state='complete';
    order.push('native-accepted');
    return options.response || {ok:true,queued:true,message:'Saved'};
  });
  const context={URL,Promise,Set,Date,chrome:api,...(firefox?{browser:api}:{})};
  vm.createContext(context);vm.runInContext(nativeCode,context);vm.runInContext(backgroundCode,context);
  const item={id:7,url:'https://files.example.com/archive.zip',finalUrl:'https://files.example.com/archive.zip',referrer:'https://site.example/page',state:'in_progress',incognito:false,paused:false,danger:'safe'};
  return {context,events,order,sent,cancelled,storage,current,item};
}
(async()=>{
  let cases=0;
  for(const firefox of [false,true]) {
    let e=environment(firefox); await e.events.download(e.item); assert.equal(e.sent.length,1); assert.equal(e.sent[0].message.automatic,true); assert.deepEqual(e.cancelled,[7]); assert(e.order.indexOf('pause')<e.order.indexOf('send'));assert(e.order.indexOf('native-accepted')<e.order.indexOf('cancel')); cases++;
    for(const [options,change] of [
      [{automatic:false},{}],[{}, {incognito:true}],[{}, {paused:true}],[{}, {state:'complete'}],[{}, {danger:'unwanted'}],
      [{}, {url:'blob:https://example.com/a',finalUrl:'blob:https://example.com/a'}],[{}, {finalUrl:'https://name:secret@example.com/f'}],
      [{exclusions:['example.com']},{}],[{exclusions:['site.example']},{}],[{}, {byExtensionId:'idx-test'}]
    ]) {e=environment(firefox,options);await e.events.download({...e.item,...change});assert.equal(e.sent.length,0);assert.equal(e.cancelled.length,0);assert(!e.order.includes('pause'));cases++;}
    for(const options of [{nativeFails:true},{response:{ok:true}},{response:{ok:true,queued:false}},{response:{ok:true,queued:"true"}},{response:{ok:false,error:'disk full'}}]) {e=environment(firefox,options);await e.events.download(e.item);assert.equal(e.cancelled.length,0);assert.equal(e.current.state,'in_progress');assert.equal(e.current.paused,false);assert(e.order.includes('resume'));assert.equal(e.storage.lastResult.error,true);cases++;}
    e=environment(firefox,{cancelFails:true});await e.events.download(e.item);assert.equal(e.cancelled.length,0);assert.match(e.storage.lastResult.message,/still downloading/);cases++;
    e=environment(firefox,{completeDuringHandoff:true});await e.events.download(e.item);assert.equal(e.cancelled.length,0);assert.match(e.storage.lastResult.message,/finished/);cases++;
    let resolve;const gate=new Promise(r=>resolve=r);e=environment(firefox,{gate});const pending=e.events.download(e.item);await new Promise(setImmediate);assert.equal(e.sent.length,1);assert.equal(e.cancelled.length,0);await e.events.download(e.item);assert.equal(e.sent.length,1);resolve();await pending;assert.deepEqual(e.cancelled,[7]);cases++;
    e=environment(firefox);assert.deepEqual(Array.from(e.context.IDX.normalizeExclusions('Example.com\n*.cdn.example.com\nhttps://site.example/path')),['example.com','cdn.example.com','site.example']);assert.equal(e.context.IDX.isExcluded('https://notexample.com/f',['example.com']),false);cases++;
    e=environment(firefox);delete e.storage.automatic;assert.equal((await e.context.IDX.getSettings()).automatic,true);cases++;
    e=environment(firefox,{pauseFails:true});await e.events.download(e.item);assert.deepEqual(e.cancelled,[7]);cases++;
  }
  console.log(`${cases} automatic-capture mock scenarios passed across Chromium and Firefox.`);
})().catch(error=>{console.error(error);process.exitCode=1;});
