#!/usr/bin/env node
/**
 * vibe-pilot launcher — resolves the platform `vp` binary and execs it.
 * The binary is fetched at install time by scripts/postinstall.js.
 * Zero npm dependencies, on purpose.
 */
'use strict';

const { spawnSync } = require('child_process');
const fs = require('fs');
const path = require('path');

const EXE = process.platform === 'win32' ? 'vp.exe' : 'vp';

function candidatePaths() {
  const vendor = path.join(__dirname, 'vendor', `${process.platform}-${process.arch}`, EXE);
  const list = [vendor];
  // VP_BINARY lets you point at a preinstalled vp (e.g. cargo-installed).
  if (process.env.VP_BINARY) list.push(process.env.VP_BINARY);
  return list;
}

const bin = candidatePaths().find((p) => p && fs.existsSync(p));

if (!bin) {
  const tag = require('../package.json').version;
  console.error(`vibe-pilot: no ${EXE} binary for ${process.platform}-${process.arch}.`);
  console.error('');
  console.error('This package ships prebuilt binaries for:');
  console.error('  macOS arm64/x64 · Linux arm64/x64 · Windows x64');
  console.error('');
  console.error('Fixes:');
  console.error(`  1. reinstall and watch for postinstall errors: npm i -g vibe-pilot@${tag}`);
  console.error('  2. build from source (zero deps, ~seconds):  cargo install --git https://github.com/Gatsuke/vibe-pilot');
  console.error('  3. or use the curl installer:               curl -fsSL https://raw.githubusercontent.com/Gatsuke/vibe-pilot/main/install.sh | bash');
  process.exit(1);
}

const result = spawnSync(bin, process.argv.slice(2), {
  stdio: 'inherit',
  windowsHide: true,
});

if (result.error) {
  console.error(`vibe-pilot: failed to launch binary: ${result.error.message}`);
  process.exit(1);
}

// Killed by signal (e.g. Ctrl-C in a pipe) — mirror conventional exit codes.
if (result.signal) process.exit(128 + (result.signal ? 1 : 0));
process.exit(result.status === null ? 1 : result.status);
