#!/usr/bin/env node
/**
 * vibe-pilot postinstall — fetches the prebuilt `vp` binary for this platform
 * from the GitHub release. No npm dependencies, on purpose.
 *
 * Escape hatches:
 *   VP_SKIP_DOWNLOAD=1   skip entirely (binary supplied some other way)
 *   VP_BINARY=/path/vp   use an existing binary, skip download
 */
'use strict';

const { spawnSync } = require('child_process');
const fs = require('fs');
const path = require('path');
const pkg = require('../package.json');

const REPO = 'Gatsuke/vibe-pilot';
const TAG = `v${pkg.version}`;
const VENDOR = path.join(__dirname, '..', 'bin', 'vendor', `${process.platform}-${process.arch}`);
const EXE = process.platform === 'win32' ? 'vp.exe' : 'vp';

function log(msg) { console.log(`\x1b[32m[vp]\x1b[0m ${msg}`); }
function warn(msg) { console.warn(`\x1b[33m[vp]\x1b[0m ${msg}`); }
function die(msg, hints) {
  console.error(`\x1b[31m[vp]\x1b[0m ${msg}`);
  for (const h of hints || []) console.error(`    ${h}`);
  process.exit(1);
}

// -- nothing to do? ----------------------------------------------------------

if (process.env.VP_SKIP_DOWNLOAD) {
  log('VP_SKIP_DOWNLOAD set — skipping binary download.');
  process.exit(0);
}

if (process.env.VP_BINARY && fs.existsSync(process.env.VP_BINARY)) {
  log(`using VP_BINARY=${process.env.VP_BINARY}`);
  process.exit(0);
}

if (fs.existsSync(path.join(VENDOR, EXE))) {
  log(`binary already present (${VENDOR}) — skipping download.`);
  process.exit(0);
}

// -- platform map ------------------------------------------------------------

const PLATFORMS = {
  'darwin-x64': 'vp-x86_64-macos.tar.gz',
  'darwin-arm64': 'vp-aarch64-macos.tar.gz',
  'linux-x64': 'vp-x86_64-linux.tar.gz',
  'linux-arm64': 'vp-aarch64-linux.tar.gz',
  'win32-x64': 'vp-x86_64-windows.zip',
};

const key = `${process.platform}-${process.arch}`;
const archive = PLATFORMS[key];

if (!archive) {
  die(`no prebuilt binary for ${key}.`, [
    'build from source instead (zero deps, compiles in seconds):',
    '  cargo install --git https://github.com/Gatsuke/vibe-pilot',
    'or on macOS/Linux:',
    '  curl -fsSL https://raw.githubusercontent.com/Gatsuke/vibe-pilot/main/install.sh | bash',
  ]);
}

// -- download ----------------------------------------------------------------

const url = `https://github.com/${REPO}/releases/download/${TAG}/${archive}`;

async function download() {
  if (typeof fetch !== 'function') {
    die('this Node is too old for vibe-pilot (needs >= 18).', [
      'upgrade Node, or build from source:',
      '  cargo install --git https://github.com/Gatsuke/vibe-pilot',
    ]);
  }

  let res;
  try {
    res = await fetch(url, { redirect: 'follow' });
  } catch (err) {
    die(`download failed: ${err.message}`, [
      'check your network / proxy, then retry:',
      `  npm i -g vibe-pilot@${pkg.version}`,
      'or build from source:',
      '  cargo install --git https://github.com/Gatsuke/vibe-pilot',
    ]);
  }

  if (res.status === 404) {
    die(`binary not found: ${archive}`, [
      `the release ${TAG} may still be uploading — try again in a minute:`,
      `  npm i -g vibe-pilot@${pkg.version}`,
      'or build from source:',
      '  cargo install --git https://github.com/Gatsuke/vibe-pilot',
    ]);
  }

  if (!res.ok) {
    die(`download failed: HTTP ${res.status} for ${url}`);
  }

  const buf = Buffer.from(await res.arrayBuffer());
  fs.mkdirSync(VENDOR, { recursive: true });
  const archivePath = path.join(VENDOR, archive);
  fs.writeFileSync(archivePath, buf);
  log(`downloaded ${archive} (${(buf.length / 1024).toFixed(0)} kB)`);
  return archivePath;
}

// -- extract -----------------------------------------------------------------

function extract(archivePath) {
  const isZip = archivePath.endsWith('.zip');
  let r;

  if (isZip) {
    // Windows: PowerShell's Expand-Archive ships with every supported Windows.
    r = spawnSync('powershell.exe', [
      '-NoProfile', '-NonInteractive', '-Command',
      `Expand-Archive -LiteralPath '${archivePath}' -DestinationPath '${VENDOR}' -Force`,
    ], { stdio: 'pipe' });
  } else {
    // macOS/Linux: tar is universal.
    r = spawnSync('tar', ['-xzf', archivePath, '-C', VENDOR], { stdio: 'pipe' });
  }

  if (r.status !== 0) {
    die(`extraction failed: ${(r.stderr && r.stderr.toString().trim()) || 'unknown error'}`);
  }

  fs.unlinkSync(archivePath);

  const binPath = path.join(VENDOR, EXE);
  if (!fs.existsSync(binPath)) {
    die(`archive did not contain ${EXE} — this is a packaging bug,`,
      ['please open an issue: https://github.com/Gatsuke/vibe-pilot/issues']);
  }
  fs.chmodSync(binPath, 0o755);
  log(`installed ${EXE} -> ${VENDOR}`);
}

// -- go ----------------------------------------------------------------------

download()
  .then(extract)
  .then(() => {
    log(`ready. try: vp report`);
  })
  .catch((err) => die(`unexpected error: ${err && err.stack ? err.stack : err}`));
