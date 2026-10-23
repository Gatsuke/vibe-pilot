// Renders vibe-pilot brand assets with Playwright (pixel-perfect text,
// local Sarasa Mono font). Run: node scripts/render-assets.js
const path = require('path');
const { chromium } = require('playwright');

const ROOT = path.resolve(__dirname, '..');

(async () => {
  const browser = await chromium.launch();

  // ---- OG banner 1280x640 ----
  let page = await browser.newPage({ viewport: { width: 1280, height: 640 }, deviceScaleFactor: 1 });
  await page.goto('file://' + path.join(ROOT, 'assets', 'og-banner.html'));
  await page.waitForTimeout(300);
  await page.screenshot({ path: path.join(ROOT, 'assets', 'og-banner.png') });
  await page.close();

  // ---- logo 512 + 1024 ----
  page = await browser.newPage({ viewport: { width: 512, height: 512 }, deviceScaleFactor: 1 });
  await page.goto('file://' + path.join(ROOT, 'assets', 'logo.svg'));
  await page.waitForTimeout(200);
  await page.screenshot({ path: path.join(ROOT, 'assets', 'logo.png'), omitBackground: true });
  await page.close();

  page = await browser.newPage({ viewport: { width: 512, height: 512 }, deviceScaleFactor: 2 });
  await page.goto('file://' + path.join(ROOT, 'assets', 'logo.svg'));
  await page.waitForTimeout(200);
  await page.screenshot({ path: path.join(ROOT, 'assets', 'logo@2x.png'), omitBackground: true });
  await page.close();

  // ---- README demo terminal card (real vp report output) ----
  page = await browser.newPage({ viewport: { width: 920, height: 500 }, deviceScaleFactor: 2 });
  await page.goto('file://' + path.join(ROOT, 'assets', 'demo-report.html'));
  await page.waitForTimeout(300);
  await page.screenshot({ path: path.join(ROOT, 'assets', 'demo-report.png'), fullPage: true });
  await page.close();

  await browser.close();
  console.log('assets rendered: og-banner.png, logo.png, logo@2x.png, demo-report.png');
})();
