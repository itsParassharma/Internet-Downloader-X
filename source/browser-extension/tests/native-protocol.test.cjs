const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const assert = require('node:assert/strict');
const code = fs.readFileSync(path.join(__dirname, '../chromium/native.js'), 'utf8');
function environment(response) {
  const messages = [];
  const chrome = {runtime:{sendNativeMessage(host, message, callback) {
    assert.equal(host, 'org.idx.downloader'); messages.push(message); callback(response);
  }}};
  const context = {chrome, URL}; vm.createContext(context); vm.runInContext(code, context);
  return {idx:context.IDX, messages};
}
(async()=>{
  let e=environment({ok:true,protocol:2,version:'0.2.0'});
  assert.equal(await e.idx.checkConnection(),'0.2.0');
  assert.equal(JSON.stringify(e.messages), '[{"action":"ping"}]');
  let cases=1;
  for(const response of [{ok:true,protocol:1,version:'0.1.0'},{ok:true,protocol:2},{ok:false,error:'missing host'},null]) {
    e=environment(response);await assert.rejects(()=>e.idx.checkConnection()); cases++;
  }
  for(const queued of [undefined,false,'true']) {
    e=environment({ok:true,queued});await assert.rejects(()=>e.idx.send('https://example.com/file'));cases++;
  }
  e=environment({ok:true,queued:true});
  await e.idx.send('https://example.com/file',false,'https://site.example/page');
  assert.equal(e.messages[0].parent_page,'https://site.example/page');cases++;
  for(const parent of ['chrome://settings','https://user:secret@example.com','https://example.com/\nprivate']) {
    e=environment({ok:true,queued:true});await e.idx.send('https://example.com/file',true,parent);
    assert.equal('parent_page' in e.messages[0],false);cases++;
  }
  console.log(`${cases} connection, acknowledgement and provenance checks passed.`);
})().catch(error=>{console.error(error);process.exitCode=1;});
