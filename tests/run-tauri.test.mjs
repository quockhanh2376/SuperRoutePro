import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';

import {
  buildTauriEnvironment,
  buildTauriInvocation,
  findLocalCargoBin,
  normalizeWindowsWorkingDirectory,
} from '../scripts/run-tauri.mjs';

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
    env: {
      USERPROFILE: 'C:\\Users\\DevUser',
      Path: 'C:\\Windows\\System32',
    },
    platform: 'win32',
    pathExists(candidate) {
      return candidate === path.join('C:\\Users\\DevUser', '.cargo', 'bin', 'cargo.exe');
    },
  });

  assert.equal(invocation.cwd, 'E:\\SuperrRoutePro');
  assert.equal(invocation.command, process.execPath);
  assert.deepEqual(invocation.args, [
    path.join('E:\\SuperrRoutePro', 'node_modules', '@tauri-apps', 'cli', 'tauri.js'),
    'dev',
  ]);
  assert.match(invocation.env.Path, /^C:\\Users\\DevUser\\\.cargo\\bin;/);
});

test('findLocalCargoBin resolves cargo from the user profile on Windows', () => {
  const cargoBin = findLocalCargoBin(
    {
      USERPROFILE: 'C:\\Users\\DevUser',
      Path: 'C:\\Windows\\System32',
    },
    {
      platform: 'win32',
      pathExists(candidate) {
        return candidate === path.join('C:\\Users\\DevUser', '.cargo', 'bin', 'cargo.exe');
      },
    },
  );

  assert.equal(cargoBin, path.join('C:\\Users\\DevUser', '.cargo', 'bin'));
});

test('buildTauriEnvironment prepends local cargo bin without mutating the original env', () => {
  const env = {
    USERPROFILE: 'C:\\Users\\DevUser',
    Path: 'C:\\Windows\\System32',
  };

  const nextEnv = buildTauriEnvironment(env, {
    platform: 'win32',
    pathExists(candidate) {
      return candidate === path.join('C:\\Users\\DevUser', '.cargo', 'bin', 'cargo.exe');
    },
  });

  assert.equal(env.Path, 'C:\\Windows\\System32');
  assert.equal(
    nextEnv.Path,
    `${path.join('C:\\Users\\DevUser', '.cargo', 'bin')};C:\\Windows\\System32`,
  );
});
