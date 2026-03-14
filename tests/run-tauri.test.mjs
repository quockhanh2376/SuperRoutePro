import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';

import { buildTauriInvocation, normalizeWindowsWorkingDirectory } from '../scripts/run-tauri.mjs';

test('normalizeWindowsWorkingDirectory strips Win32 device prefix from drive paths', () => {
  assert.equal(
    normalizeWindowsWorkingDirectory('\\\\?\\E:\\SuperrRoutePro'),
    'E:\\SuperrRoutePro',
  );
});

test('normalizeWindowsWorkingDirectory strips Win32 device prefix from UNC paths', () => {
  assert.equal(
    normalizeWindowsWorkingDirectory('\\\\?\\UNC\\server\\share\\repo'),
    '\\\\server\\share\\repo',
  );
});

test('buildTauriInvocation uses normalized cwd and local tauri cli script', () => {
  const invocation = buildTauriInvocation(['dev'], {
    cwd: '\\\\?\\E:\\SuperrRoutePro',
    packageRoot: '\\\\?\\E:\\SuperrRoutePro',
  });

  assert.equal(invocation.cwd, 'E:\\SuperrRoutePro');
  assert.equal(invocation.command, process.execPath);
  assert.deepEqual(invocation.args, [
    path.join('E:\\SuperrRoutePro', 'node_modules', '@tauri-apps', 'cli', 'tauri.js'),
    'dev',
  ]);
});
