// Renders each view at three window sizes in the synthetic in-browser engine
// (no Tauri, no network needed except map tiles) and writes PNGs to
// docs/screenshots/. Usage: npm run build && node scripts/screenshots.mjs [filter]
import { spawn } from 'node:child_process';
import { mkdirSync } from 'node:fs';
import puppeteer from 'puppeteer-core';

const CHROME = process.env.CHROME_PATH || '/usr/bin/google-chrome';
const PORT = 4173;
const BASE = `http://127.0.0.1:${PORT}`;
const OUT = 'docs/screenshots';
const filter = process.argv[2] || '';
mkdirSync(OUT, { recursive: true });

const server = spawn('npx', ['vite', 'preview', '--port', String(PORT), '--strictPort', '--host', '127.0.0.1'], { stdio: 'ignore' });
await new Promise((r) => setTimeout(r, 2500));

const proxy = process.env.HTTPS_PROXY || process.env.https_proxy;
const args = ['--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist', '--ignore-certificate-errors', '--hide-scrollbars'];
if (proxy) args.push(`--proxy-server=${proxy}`, '--proxy-bypass-list=127.0.0.1;localhost');
const browser = await puppeteer.launch({ executablePath: CHROME, headless: 'new', args });

const sizes = { small: [480, 360], desktop: [1280, 800], ultrawide: [2560, 1080] };
const jobs = [];
for (const view of ['radar', 'map', 'sky']) for (const [s, wh] of Object.entries(sizes)) jobs.push({ name: `${view}-${s}`, q: `view=${view}&theme=phosphor&home=1&t=96`, wh, wait: view === 'map' ? 7000 : 4200 });
jobs.push({ name: 'radar-amber', q: 'view=radar&theme=amber&home=1&t=96', wh: sizes.desktop, wait: 4200 });
jobs.push({ name: 'radar-daylight', q: 'view=radar&theme=daylight&home=1&t=96', wh: sizes.desktop, wait: 4200 });
jobs.push({ name: 'sky-daylight', q: 'view=sky&theme=daylight&home=1&t=96', wh: sizes.desktop, wait: 4200 });
jobs.push({ name: 'radar-small-panel', q: 'view=radar&theme=phosphor&home=1&t=96&panel=1', wh: sizes.small, wait: 4200 });
jobs.push({ name: 'radar-alert', q: 'view=radar&theme=phosphor&home=1&t=96&alerts=1', wh: sizes.desktop, wait: 4800 });
jobs.push({ name: 'onboarding', q: 'view=radar&theme=phosphor&fresh=1', wh: sizes.desktop, wait: 6000 });
jobs.push({ name: 'design-phosphor', q: 'theme=phosphor', hash: '#/design', wh: [1280, 2000], wait: 2500 });
jobs.push({ name: 'design-amber', q: 'theme=amber', hash: '#/design', wh: [1280, 2000], wait: 2500 });
jobs.push({ name: 'design-daylight', q: 'theme=daylight', hash: '#/design', wh: [1280, 2000], wait: 2500 });
const tab = (label) => async (p) => {
  await p.evaluate(() => [...document.querySelectorAll('button')].find((b) => b.getAttribute('aria-label') === 'Settings')?.click());
  await new Promise((r) => setTimeout(r, 400));
  await p.evaluate((l) => [...document.querySelectorAll('[role=tab]')].find((b) => b.textContent.trim().toLowerCase() === l)?.click(), label);
};
jobs.push({ name: 'settings-display', q: 'view=radar&theme=amber&home=1&t=96', wh: [900, 760], wait: 3000, act: tab('display') });
jobs.push({ name: 'settings-alerts', q: 'view=radar&theme=amber&home=1&t=96', wh: [900, 760], wait: 3000, act: tab('alerts') });
jobs.push({ name: 'settings', q: 'view=radar&theme=phosphor&home=1&t=96', wh: sizes.desktop, wait: 3000, act: async (p) => { await p.evaluate(() => [...document.querySelectorAll('button')].find((b) => b.getAttribute('aria-label') === 'Settings')?.click()); } });
jobs.push({ name: 'today', q: 'view=radar&theme=phosphor&home=1&t=96', wh: sizes.desktop, wait: 3000, act: async (p) => { await p.evaluate(() => [...document.querySelectorAll('button')].find((b) => b.getAttribute('aria-label') === 'Today')?.click()); } });

const errors = [];
for (const j of jobs) {
  if (filter && !j.name.includes(filter)) continue;
  const page = await browser.newPage();
  page.on('pageerror', (e) => errors.push(`${j.name}: ${e.message}`));
  page.on('console', (m) => { if (m.type() === 'error' && !/tiles|Failed to load resource|net::/.test(m.text())) errors.push(`${j.name}: console ${m.text()}`); });
  await page.setViewport({ width: j.wh[0], height: j.wh[1], deviceScaleFactor: 1 });
  await page.goto(`${BASE}/?${j.q}${j.hash ?? ''}`, { waitUntil: 'load' });
  await new Promise((r) => setTimeout(r, 800));
  if (j.act) await j.act(page);
  await new Promise((r) => setTimeout(r, j.wait));
  await page.screenshot({ path: `${OUT}/${j.name}.png` });
  console.log('wrote', j.name);
  await page.close();
}
await browser.close();
server.kill();
if (errors.length) { console.log('\nPAGE ERRORS:\n' + [...new Set(errors)].join('\n')); }
