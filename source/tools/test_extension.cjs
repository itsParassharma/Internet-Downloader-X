// Real Chromium -> installed native host -> Rust download engine acceptance.
// Install Playwright separately; this is development tooling, never in the store ZIP.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const cp = require('node:child_process');
const http = require('node:http');
const crypto = require('node:crypto');
const source = path.resolve(__dirname, '..');
const workspace = path.resolve(source, '../..');
process.env.PLAYWRIGHT_BROWSERS_PATH = path.join(workspace, '.build-cache/browsers');
const { chromium } = require(path.join(workspace, '.build-cache/browser-tests/node_modules/playwright'));
const run = path.join(source, '.migration-backup/acceptance', `run-${Date.now()}`);
fs.mkdirSync(run, {recursive: true});
const data = path.join(run, 'data');
process.env.IDX_DATA_DIR = data;
process.env.IDX_INSTANCE_PORT = '47847';
const extension = path.join(source, 'browser-extension/chromium');
const unpacked=path.join(run,'extension');fs.cpSync(extension,unpacked,{recursive:true});
const installer = path.join(source, '.migration-backup/acceptance/IDX-TestSetup.exe');
const install = path.join(run, 'installed');
const version = JSON.parse(fs.readFileSync(path.join(extension, 'manifest.json'), 'utf8')).version;
const backup = path.join(run, 'registry-before.json');
const report = { run, install, data, checks: [], extension_id: null, screenshots: [] };
report.checks.push=(...items)=>{console.log(...items);return Array.prototype.push.apply(report.checks,items);};
const shortcuts = ['IDX - Internet Downloader X.lnk', 'Uninstall IDX.lnk'].map(name =>
  path.join(process.env.APPDATA, 'Microsoft/Windows/Start Menu/Programs/IDX - Internet Downloader X', name));
const shortcutBytes = shortcuts.map(file => fs.existsSync(file) ? fs.readFileSync(file) : null);
const payload = Buffer.from('IDX real browser and native host acceptance fixture\n'.repeat(4096));
let videoPayload=payload;
const mediaRequests=[];
const streamRoot=path.join(run,'stream-fixtures');
cp.execFileSync('python',[path.join(__dirname,'generate_stream_fixtures.py'),streamRoot],{windowsHide:true});
const streamRequests=[];
function waitDownload(predicate,timeout=45000){return until(()=>{try{const item=JSON.parse(fs.readFileSync(path.join(data,'library.json'),'utf8')).downloads.find(predicate);if(item?.status==='Error')throw new Error(item.error);return item?.status==='Complete'?item:null;}catch(error){if(error.message && !/ENOENT|JSON|Unexpected/.test(error.message))throw error;return null;}},timeout);}
function verifyMovie(file){
 const probe=JSON.parse(cp.execFileSync(path.join(source,'media-tools/ffprobe.exe'),['-v','error','-show_streams','-show_format','-of','json',file],{windowsHide:true,encoding:'utf8'}));
 assert(probe.streams.some(s=>s.codec_type==='video'));assert(probe.streams.some(s=>s.codec_type==='audio'));assert(Number(probe.format.duration)>=2.9);
 const hashes=input=>cp.execFileSync(path.join(source,'media-tools/ffmpeg.exe'),['-v','error','-i',input,'-map','0:v:0','-f','framemd5','-'],{windowsHide:true,encoding:'utf8'}).split(/\r?\n/).filter(line=>line && !line.startsWith('#')).map(line=>line.split(',').at(-1).trim());
 assert.deepEqual(hashes(file),hashes(path.join(streamRoot,'source.mp4')));
}
const server = http.createServer((req, res) => {
  if(req.url.startsWith('/streams/')) {
    const route=new URL(req.url,'http://localhost').pathname.slice('/streams/'.length);
    const file=path.resolve(streamRoot,route);
    if(!file.startsWith(streamRoot+path.sep)||!fs.existsSync(file)||!fs.statSync(file).isFile()){res.writeHead(404);res.end();return;}
    const authenticated=(req.headers.cookie||'').includes('idx_fixture_session=synthetic-secret');
    streamRequests.push({route,authenticated,range:req.headers.range||'',agent:req.headers['user-agent']||''});
    if(!authenticated){res.writeHead(403);res.end('Login required');return;}
    const body=fs.readFileSync(file);let start=0,end=body.length-1;const range=/^bytes=(\d+)-(\d+)$/.exec(req.headers.range||'');
    if(range){start=Number(range[1]);end=Number(range[2]);if(end>=body.length){res.writeHead(416);res.end();return;}res.statusCode=206;res.setHeader('Content-Range',`bytes ${start}-${end}/${body.length}`);}
    res.setHeader('Content-Type',file.endsWith('.m3u8')?'application/vnd.apple.mpegurl':file.endsWith('.mpd')?'application/dash+xml':'application/octet-stream');
    res.setHeader('Content-Length',end-start+1);res.end(body.subarray(start,end+1));return;
  }
  if(req.url==='/private.bin'){
    const authenticated=(req.headers.cookie||'').includes('idx_fixture_session=synthetic-secret');streamRequests.push({route:'private.bin',authenticated});
    if(!authenticated){res.writeHead(403);res.end();return;}res.setHeader('Content-Type','application/octet-stream');res.setHeader('Content-Disposition','attachment; filename="private.bin"');res.setHeader('Content-Length',payload.length);res.end(payload);return;
  }
  if (req.url==='/player') {
    res.setHeader('Content-Type','text/html');res.end(`<!doctype html><title>IDX browser acceptance video</title><style>body{font:16px Arial;margin:40px;background:#fafafa}video{display:block;width:640px;height:360px;background:#203448;margin:32px 0}iframe{width:400px;height:220px;border:1px solid #aaa}</style><h1>Browser integration fixture</h1><p>Local test files for IDX. No external media.</p><video id="clip" controls src="/clip.webm"></video><a id="automatic-link" href="/automatic.bin">Download test file</a><iframe src="/frame"></iframe>`);return;
  }
  if(req.url==='/frame') {res.setHeader('Content-Type','text/html');res.end('<video controls src="/frame.webm" style="width:360px;height:180px;background:#203448"></video>');return;}
  if (!['/fixture.bin', '/unmatched.dat','/automatic.bin','/fallback.bin','/clip.webm','/frame.webm','/dynamic.webm','/stream.m3u8','/opaque'].includes(req.url)) { res.writeHead(404); res.end(); return; }
  const body=/webm|opaque/.test(req.url)?videoPayload:payload;
  let start = 0, end = body.length - 1;
  const range = /^bytes=(\d+)-(\d*)$/.exec(req.headers.range || '');
  if (range) { start = Number(range[1]); if(range[2]) end = Math.min(end, Number(range[2])); }
  res.setHeader('Accept-Ranges', 'bytes'); res.setHeader('ETag', '"idx-test-v1"');
  const media=/mp4|webm|opaque/.test(req.url);
  res.setHeader('Content-Type', media ? 'video/webm' : 'application/octet-stream');
  if(media) mediaRequests.push({url:req.url,referer:req.headers.referer || '',agent:req.headers['user-agent'] || ''});
  if(['/automatic.bin','/fallback.bin'].includes(req.url))res.setHeader('Content-Disposition',`attachment; filename="${req.url.slice(1)}"`);
  res.setHeader('Content-Length', end - start + 1);
  if(range) { res.statusCode = 206; res.setHeader('Content-Range', `bytes ${start}-${end}/${body.length}`); }
  if(['/automatic.bin','/fallback.bin'].includes(req.url) && req.method!=='HEAD') {
    let at=start;const timer=setInterval(()=>{if(at>end){clearInterval(timer);res.end();return;}const next=Math.min(at+4096,end+1);res.write(payload.subarray(at,next));at=next;},10);res.on('close',()=>clearInterval(timer));
  }else res.end(req.method === 'HEAD' ? undefined : body.subarray(start, end + 1));
});
async function until(fn, timeout=15000) {
  const deadline = Date.now()+timeout;
  while(Date.now()<deadline) { const value=await fn(); if(value) return value; await new Promise(r=>setTimeout(r,100)); }
  throw new Error('Timed out waiting for acceptance condition');
}
(async()=>{
  let context, backedUp=false;
  try {
    await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
    fs.mkdirSync(data, {recursive:true});
    fs.writeFileSync(path.join(data,'library.json'), JSON.stringify({version:1,next_id:1,downloads:[],settings:{download_dir:path.join(run,'downloads'),categories:[{name:'Browser files',extensions:'bin',sites_only:true,sites:'127.0.0.1',folder:path.join(run,'category-files')},{name:'Other',fallback:true}]}}));
    cp.execFileSync('python', [path.join(__dirname,'test_registry.py'),'save',backup], {windowsHide:true}); backedUp=true;
    context=await chromium.launchPersistentContext(path.join(run,'profile'), {channel:'chromium',headless:true,
      env:{...process.env}, viewport:{width:1280,height:800},
      args:[`--disable-extensions-except=${unpacked}`,`--load-extension=${unpacked}`]});
    let [worker]=context.serviceWorkers();
    if(!worker)worker=await context.waitForEvent('serviceworker');
    const id=worker.url().split('/')[2]; report.extension_id=id;
    const publicKey=Buffer.from(JSON.parse(fs.readFileSync(path.join(extension,'manifest.json'),'utf8')).key,'base64');
    const expectedId=crypto.createHash('sha256').update(publicKey).digest('hex').slice(0,32).replace(/[0-9a-f]/g,x=>String.fromCharCode(97+parseInt(x,16)));
    assert.equal(id,expectedId);
    assert.match(id,/^[a-p]{32}$/); report.checks.push('Manifest V3 service worker starts in real Chromium');
    const page=await context.newPage();
    videoPayload=Buffer.from(await page.evaluate(async()=>{
      const canvas=document.createElement('canvas');canvas.width=640;canvas.height=360;const ctx=canvas.getContext('2d');
      const stream=canvas.captureStream(10),recorder=new MediaRecorder(stream,{mimeType:'video/webm;codecs=vp8'}),chunks=[];
      recorder.ondataavailable=event=>chunks.push(event.data);const done=new Promise(resolve=>recorder.onstop=resolve);recorder.start();
      for(let frame=0;frame<10;frame++){
        const gradient=ctx.createLinearGradient(0,0,640,360);gradient.addColorStop(0,'#163a58');gradient.addColorStop(1,'#248d88');ctx.fillStyle=gradient;ctx.fillRect(0,0,640,360);
        ctx.fillStyle='white';ctx.font='bold 38px Arial';ctx.fillText('A video, ready for IDX',64,170);ctx.font='18px Arial';ctx.fillText('Local playback and download test',64,207);
        ctx.fillStyle='#8ce5c3';ctx.fillRect(64,248,40+frame*38,5);await new Promise(resolve=>setTimeout(resolve,100));
      }
      recorder.stop();await done;stream.getTracks().forEach(track=>track.stop());return Array.from(new Uint8Array(await new Blob(chunks,{type:'video/webm'}).arrayBuffer()));
    }));
    const errors=[];page.on('pageerror',error=>errors.push(String(error)));
    await page.goto(`chrome-extension://${id}/popup.html`);
    assert.equal(await page.locator('#extension-id').count(),0);
    assert.equal(await page.locator('#copy-id').count(),0);
    await until(()=>page.locator('#check-connection').isEnabled());
    await page.screenshot({path:path.join(run,'extension-setup.png')});report.screenshots.push('extension-setup.png');
    cp.execFileSync(installer, ['/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART','/NOICONS','/GROUP=IDX Acceptance Test',`/DIR=${install}`,
      `/LOG=${path.join(run,'installer.log')}`], {windowsHide:true,timeout:120000});
    const afterInstall=path.join(run,'registry-after-install.json');
    cp.execFileSync('python',[path.join(__dirname,'test_registry.py'),'save',afterInstall],{windowsHide:true});
    assert.deepEqual(JSON.parse(fs.readFileSync(afterInstall)),JSON.parse(fs.readFileSync(backup)));
    report.checks.push('Installer requires no extension ID and leaves browser registration unchanged');
    fs.writeFileSync(path.join(data,'browser-setup.json'),JSON.stringify({offered:true,extension_ids:[]}));
    cp.spawn(path.join(install,'idx.exe'),[],{windowsHide:true,env:{...process.env},stdio:'ignore'}).unref();
    await until(()=>fs.existsSync(path.join(install,'browser-extension/native-hosts/chrome.json')));
    cp.execFileSync(path.join(install,'idx.exe'),['--connect-browser'],{windowsHide:true,env:{...process.env}});
    const setup=JSON.parse(fs.readFileSync(path.join(data,'browser-setup.json')));
    assert.deepEqual(setup.extension_ids,[id]);
    setup.offered=true;fs.writeFileSync(path.join(data,'browser-setup.json'),JSON.stringify(setup));
    report.checks.push('Opening IDX configures its bundled identity automatically; copied extension folder retains same identity; no copied ID or popup required');
    for(const browser of ['chrome','edge']) {
      const manifest=JSON.parse(fs.readFileSync(path.join(install,'browser-extension/native-hosts',`${browser}.json`),'utf8'));
      assert.deepEqual(manifest.allowed_origins,[`chrome-extension://${id}/`]);
      assert(fs.existsSync(path.resolve(install,'browser-extension/native-hosts',manifest.path)));
    }
    report.checks.push('IDX app connection handler creates valid Chrome and Edge manifests without PowerShell; reconnect is idempotent');
    report.binary_sha256 = {};
    for(const name of ['idx.exe', 'idx-native-host.exe']) {
      const hash = file => crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
      report.binary_sha256[name] = hash(path.join(install, name));
      assert.equal(report.binary_sha256[name], hash(path.join(source, 'target/release', name)));
    }
    report.checks.push('Both installed binaries match the current release build SHA-256');
    shortcuts.forEach((file, i) => assert.deepEqual(fs.existsSync(file) ? fs.readFileSync(file) : null, shortcutBytes[i]));
    report.checks.push('Isolated installer leaves existing IDX Start Menu shortcuts unchanged');
    await page.locator('#check-connection').click();
    await until(async()=> (await page.locator('#connection-status').textContent()).includes(`Connected to IDX ${version}`));
    report.checks.push(`Real Chromium native-messaging ping returns IDX ${version}`);
    assert(!(fs.existsSync(path.join(data,'inbox')) && fs.readdirSync(path.join(data,'inbox')).length));
    report.checks.push('Connection checks queue no downloads');
    await page.locator('#automatic-options').click();
    await page.locator('#excluded-sites').fill('Example.com\n*.cdn.example.com');
    await page.locator('#save-settings').click();
    await until(async()=> (await page.locator('#settings-status').textContent()).startsWith('Saved.'));
    await page.reload();await page.locator('#automatic-options').click();
    await until(async()=> (await page.locator('#excluded-sites').inputValue()).includes('cdn.example.com'));
    await until(async()=> (await page.locator('#connection-status').textContent()).includes(`Connected to IDX ${version}`));
    assert.equal(await page.locator('#automatic').isChecked(),true);
    report.checks.push('Exclusions persist after reload; automatic capture is on by default');
    const url=`http://127.0.0.1:${server.address().port}/fixture.bin`;
    const parent=`http://127.0.0.1:${server.address().port}/downloads-page`;
    await page.locator('#download-url').fill('https://example.com/archive.zip');
    await page.screenshot({path:path.join(run,'extension-connected.png')});report.screenshots.push('extension-connected.png');
    await page.setViewportSize({width:401,height:600});
    assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
    await page.screenshot({path:path.join(run,'extension-popup.png')});report.screenshots.push('extension-popup.png');
    report.checks.push('Popup fits 401-pixel width without horizontal overflow');
    const reply=await worker.evaluate(async ({url,parent})=>IDX.send(url,true,parent),{url,parent});
    assert.equal(reply,'Download saved in IDX.');
    const download=await until(()=>{
      try {const library=JSON.parse(fs.readFileSync(path.join(data,'library.json'),'utf8'));return library.downloads.find(d=>d.url===url && d.status==='Complete');}catch{return null;}
    },30000);
    assert.equal(download.category,'Browser files');
    assert.equal(download.path,path.join(run,'category-files/fixture.bin'));
    assert.equal(download.parent_page,parent);
    const fallbackUrl=`http://127.0.0.1:${server.address().port}/unmatched.dat`;
    await worker.evaluate(async url=>IDX.send(url,true,''),fallbackUrl);
    const fallback=await until(()=>{try{return JSON.parse(fs.readFileSync(path.join(data,'library.json'),'utf8')).downloads.find(d=>d.url===fallbackUrl && d.status==='Complete');}catch{return null;}},30000);
    assert.equal(fallback.category,'Other');
    assert.equal(fallback.path,path.join(run,'downloads/unmatched.dat'));
    assert.deepEqual(fs.readFileSync(fallback.path),payload);
    report.checks.push('Live browser downloads use extension/site category folder and unmatched files retain default folder');
    const received=fs.readFileSync(download.path);assert.deepEqual(received,payload);
    report.download_sha256=crypto.createHash('sha256').update(received).digest('hex');
    report.checks.push('Real extension -> native host -> durable inbox -> completed Rust HTTP download, exact bytes and source page');
    const player=await context.newPage();
    player.on('pageerror',error=>errors.push(String(error)));
    await player.goto(`http://127.0.0.1:${server.address().port}/player`);
    const mediaButton=player.locator('idx-media-panel').first().getByRole('button',{name:'Download this video with IDX',exact:true});
    await mediaButton.waitFor({state:'visible'});
    await until(async()=>player.locator('#clip').evaluate(video=>video.readyState>=2));
    await player.locator('#clip').evaluate(async video=>{video.muted=true;await video.play();});
    await until(async()=>player.locator('#clip').evaluate(video=>video.currentTime>0.25));
    await player.locator('#clip').evaluate(video=>video.pause());
    await until(()=>player.frameLocator('iframe').getByRole('button',{name:'Download this video with IDX',exact:true}).isVisible());
    await player.screenshot({path:path.join(run,'media-panel.png')});report.screenshots.push('media-panel.png');
    await mediaButton.click();
    const media=await until(()=>{try{return JSON.parse(fs.readFileSync(path.join(data,'library.json'),'utf8')).downloads.find(d=>d.url.endsWith('/clip.webm') && d.status==='Complete');}catch{return null;}},30000);
    assert.deepEqual(fs.readFileSync(media.path),videoPayload);assert.equal(path.basename(media.path),'clip.webm');
    assert.equal(media.referer,player.url());assert.equal(media.parent_page,player.url());
    assert(mediaRequests.some(request=>request.referer===player.url()));
    report.checks.push('Trusted floating-panel click -> native host -> Rust media download, exact bytes, filename and Referer');
    await player.locator('idx-media-panel').first().getByRole('button',{name:'Choose a media file'}).click();
    await player.screenshot({path:path.join(run,'media-menu.png')});report.screenshots.push('media-menu.png');
    await player.locator('#automatic-link').click();
    const automatic=await until(()=>{try{return JSON.parse(fs.readFileSync(path.join(data,'library.json'),'utf8')).downloads.find(d=>d.url.endsWith('/automatic.bin') && d.status==='Complete');}catch{return null;}},30000);
    assert.deepEqual(fs.readFileSync(automatic.path),payload);
    const browserCopy=await worker.evaluate(async()=>IDX.invoke(chrome.downloads,'search',{}));
    const intercepted=browserCopy.find(item=>item.url.endsWith('/automatic.bin'));
    assert(intercepted);assert.equal(intercepted.state,'interrupted');
    report.checks.push('Real Chrome download event automatically transfers to Rust and cancels the held browser copy after acknowledgement');
    const nativeManifest=path.join(install,'browser-extension/native-hosts/chrome.json');
    const correctManifest=fs.readFileSync(nativeManifest);
    try {
      const broken=JSON.parse(correctManifest);broken.path=path.join(run,'missing-native-host.exe');fs.writeFileSync(nativeManifest,JSON.stringify(broken));
      await player.locator('#automatic-link').evaluate(link=>link.href='/fallback.bin');
      const fallbackEvent=player.waitForEvent('download');await player.locator('#automatic-link').click();
      const fallbackDownload=await fallbackEvent;
      assert.equal(await fallbackDownload.failure(),null);
      assert.deepEqual(fs.readFileSync(await fallbackDownload.path()),payload);
      const copies=await worker.evaluate(async()=>IDX.invoke(chrome.downloads,'search',{}));
      assert.equal(copies.find(item=>item.url.endsWith('/fallback.bin')).state,'complete');
      report.checks.push('Unavailable native host resumes the held browser transfer and preserves the exact file');
    }finally{fs.writeFileSync(nativeManifest,correctManifest);}
    await player.evaluate(()=>fetch('/opaque').then(r=>r.arrayBuffer()));
    await player.locator('idx-media-panel').first().getByRole('button',{name:'Choose a media file'}).click();
    await player.locator('idx-media-panel').first().locator('.menu button').filter({hasText:'opaque'}).waitFor({state:'visible'});
    report.checks.push('Response MIME discovers media URLs without file extensions');
    await player.locator('idx-media-panel').first().locator('.menu button').filter({hasText:'opaque'}).click();
    const opaque=await until(()=>{try{return JSON.parse(fs.readFileSync(path.join(data,'library.json'),'utf8')).downloads.find(d=>d.url.endsWith('/opaque') && d.status==='Complete');}catch{return null;}},30000);
    assert.deepEqual(fs.readFileSync(opaque.path),videoPayload);assert.equal(path.extname(opaque.path),'.webm');
    report.checks.push('Opaque media URLs preserve a playable filename and exact video bytes in Rust');
    await player.evaluate(()=>{const video=document.createElement('video');video.src='/dynamic.webm';video.style.cssText='width:320px;height:180px;background:#203448';document.body.prepend(video);});
    await until(async()=>await player.locator('idx-media-panel').count()===2);
    report.checks.push('Dynamic players and embedded frames receive separate media panels');
    await page.locator('#media-panel').uncheck();await page.locator('#save-settings').click();
    await until(async()=>await mediaButton.isHidden());
    await page.locator('#media-panel').check();await page.locator('#panel-mode').selectOption('mini');await page.locator('#save-settings').click();
    await until(async()=>await mediaButton.isVisible());
    assert.equal(await player.locator('idx-media-panel').first().locator('.main span').last().textContent(),'');
    report.checks.push('Panel preferences apply live and mini mode preserves the download action');
    await context.addCookies([{name:'idx_fixture_session',value:'synthetic-secret',url:`http://127.0.0.1:${server.address().port}/`,httpOnly:true,sameSite:'Lax'}]);
    await worker.evaluate(async url=>IDX.send(url,true,''),`http://127.0.0.1:${server.address().port}/private.bin`);
    const privateFile=await waitDownload(d=>d.url.endsWith('/private.bin'));assert.deepEqual(fs.readFileSync(privateFile.path),payload);
    report.checks.push('HttpOnly browser cookies allow an authenticated direct download without storing credentials in the library');
    await page.locator('#panel-mode').selectOption('full');await page.locator('#save-settings').click();
    const streamCases=[['hls/video.m3u8','HLS TS'],['encrypted/video.m3u8','AES-128 HLS'],['split/master.m3u8','HLS separate audio/video'],['range/video.m3u8','HLS byte ranges'],['dash/main.mpd','DASH separate audio/video']];
    for(const [route,label] of streamCases){
      await player.goto(`http://127.0.0.1:${server.address().port}/player`);
      await until(async()=>player.locator('#clip').evaluate(video=>video.readyState>=2));
      await player.locator('#clip').evaluate(async video=>{video.muted=true;await video.play();});
      await until(async()=>player.locator('#clip').evaluate(video=>video.currentTime>0.25));
      await player.locator('#clip').evaluate(video=>video.pause());
      await player.evaluate(route=>fetch('/streams/'+route).then(r=>r.text()),route);
      const host=player.locator('idx-media-panel').first();await host.getByRole('button',{name:'Choose a media file'}).click();
      const button=host.locator('.menu button').filter({hasText:new RegExp((route.endsWith('.mpd')?'DASH':'HLS')+'.*'+route.split('/').at(-1).replace('.','\\.') )});
      await button.waitFor({state:'visible'});
      if(route==='split/master.m3u8'){
        await host.locator('.menu button').filter({hasText:'HLS · 180p'}).waitFor({state:'visible'});
        await player.screenshot({path:path.join(run,'stream-quality-menu.png')});report.screenshots.push('stream-quality-menu.png');
        await host.locator('.menu button').filter({hasText:'HLS · 180p'}).click();
      }else await button.click();
      const completed=await waitDownload(d=>d.url.endsWith('/streams/'+route));verifyMovie(completed.path);
      assert.equal(path.extname(completed.path),'.mp4');
      if(route==='split/master.m3u8')assert.equal(completed.stream.max_height,180);
      report.checks.push(`${label}: trusted panel click, authenticated Rust segment download, playable MP4 with audio and identical decoded video frames`);
    }
    assert(streamRequests.filter(r=>r.route!=='private.bin').every(r=>r.authenticated));
    assert(streamRequests.some(r=>r.range));
    const libraryText=fs.readFileSync(path.join(data,'library.json'),'utf8');assert(!libraryText.includes('synthetic-secret'));assert(!libraryText.includes('idx_fixture_session'));
    if(fs.existsSync(path.join(data,'inbox')))for(const file of fs.readdirSync(path.join(data,'inbox')))assert(!fs.readFileSync(path.join(data,'inbox',file),'utf8').includes('synthetic-secret'));
    report.checks.push('Stream requests respect browser session cookies; library and inbox contain no plaintext cookie secret');
    await page.locator('#panel-mode').selectOption('full');await page.locator('#save-settings').click();
    assert.deepEqual(errors,[]);report.checks.push('Popup has no JavaScript page errors');
    report.ok=true;
  } catch(error) {report.ok=false;report.error=String(error.stack||error);process.exitCode=1;}
  finally {
    if(backedUp) {cp.execFileSync('python',[path.join(__dirname,'test_registry.py'),'restore',backup],{windowsHide:true});report.registry_restored=true;}
    // Chromium's Windows process-tree cleanup waits for the native host's GUI
    // child. Stop only this run's app after its persisted result is verified.
    cp.execFileSync('powershell.exe', ['-NoProfile', '-Command',
      'Get-Process -Name idx -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $env:IDX_TEST_EXE } | Stop-Process'],
      {windowsHide:true, env:{...process.env, IDX_TEST_EXE:path.join(install,'idx.exe')}});
    if(context)await context.close();
    server.close();
    if(report.ok) {
      cp.execFileSync(path.join(install,'unins000.exe'), ['/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART'], {windowsHide:true,timeout:120000});
      await until(()=>!fs.existsSync(path.join(install,'idx.exe')));
      assert(fs.existsSync(path.join(data,'library.json')));
      assert.deepEqual(fs.readFileSync(path.join(run,'category-files/fixture.bin')), payload);
      shortcuts.forEach((file,i)=>assert.deepEqual(fs.existsSync(file)?fs.readFileSync(file):null,shortcutBytes[i]));
      report.checks.push('Uninstaller removes app files and preserves test library, completed file and existing shortcuts');
    }
    fs.writeFileSync(path.join(run,'report.json'),JSON.stringify(report,null,2));
    fs.writeFileSync(path.join(source,'.migration-backup/acceptance/latest-run.txt'),run);
    console.log(JSON.stringify(report,null,2));
  }
})();
