// Render the existing code-based IDX brand tile; no third-party image downloads.
const fs=require('node:fs'),path=require('node:path');
const {pathToFileURL}=require('node:url');
const source=path.resolve(__dirname,'..'),workspace=path.resolve(source,'../..');
process.env.PLAYWRIGHT_BROWSERS_PATH=path.join(workspace,'.build-cache/browsers');
const {chromium}=require(path.join(workspace,'.build-cache/browser-tests/node_modules/playwright'));
(async()=>{
  const store=path.join(source,'browser-extension/store');
  fs.copyFileSync(path.join(source,'browser-extension/chromium/icons/idx-128.png'),path.join(store,'icon-128.png'));
  const browser=await chromium.launch({channel:'chromium',headless:true});
  try {
    const page=await browser.newPage({viewport:{width:440,height:280},deviceScaleFactor:1});
    await page.goto(pathToFileURL(path.join(store,'promo.html')).href);
    await page.screenshot({path:path.join(store,'promo-440x280.png')});
  } finally {await browser.close();}
  const run=fs.readFileSync(path.join(source,'.migration-backup/acceptance/latest-run.txt'),'utf8').trim();
  const report=JSON.parse(fs.readFileSync(path.join(run,'report.json'),'utf8'));
  if(!report.ok)throw new Error('Store screenshots must come from a passing browser acceptance run');
  for(const name of ['setup','connected'])fs.copyFileSync(path.join(run,`extension-${name}.png`),path.join(store,`screenshot-${name}-1280x800.png`));
  for(const name of ['media-panel','media-menu','stream-quality-menu'])fs.copyFileSync(path.join(run,`${name}.png`),path.join(store,`screenshot-${name}-1280x800.png`));
  for(const name of ['privacy.html','privacy.css'])fs.copyFileSync(path.join(source,'browser-extension/chromium',name),path.join(store,name));
  fs.copyFileSync(path.join(source,'browser-extension/chromium/icons/idx-128.png'),path.join(store,'icon-128.png'));
  console.log('Rendered store tile and copied real Chromium screenshots and privacy files.');
})().catch(error=>{console.error(error);process.exitCode=1;});
