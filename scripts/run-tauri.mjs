import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const packageRoot = path.resolve(scriptDir, '..');

export function normalizeWindowsWorkingDirectory(inputPath) {
  if (process.platform !== 'win32') {
    return inputPath;
  }

  if (/^\\\\\?\\[A-Za-z]:\\/.test(inputPath)) {
    return inputPath.slice(4);
  }

  if (/^\\\\\?\\UNC\\/.test(inputPath)) {
    return `\\\\${inputPath.slice(8)}`;
  }

  return inputPath;
}

function getPathKey(env) {
  return Object.keys(env).find((key) => key.toLowerCase() === 'path') ?? 'Path';
}

function normalizePathEntry(entry, platform) {
  if (platform === 'win32') {
    return normalizeWindowsWorkingDirectory(entry).replace(/[\\/]+$/, '').toLowerCase();
  }

  return entry.replace(/\/+$/, '');
}

export function findLocalCargoBin(env = process.env, options = {}) {
  const platform = options.platform ?? process.platform;
  const pathExists = options.pathExists ?? existsSync;

  if (platform !== 'win32') {
    return null;
  }

  const candidates = [];
  const userProfile = env.USERPROFILE ? normalizeWindowsWorkingDirectory(env.USERPROFILE) : '';
  const home = env.HOME ? normalizeWindowsWorkingDirectory(env.HOME) : '';
  const homeDrive = env.HOMEDRIVE ?? '';
  const homePath = env.HOMEPATH ?? '';

  if (userProfile) {
    candidates.push(path.join(userProfile, '.cargo', 'bin'));
  }
  if (home && home !== userProfile) {
    candidates.push(path.join(home, '.cargo', 'bin'));
  }
  if (homeDrive && homePath) {
    const combinedHome = normalizeWindowsWorkingDirectory(`${homeDrive}${homePath}`);
    if (combinedHome && combinedHome !== userProfile && combinedHome !== home) {
      candidates.push(path.join(combinedHome, '.cargo', 'bin'));
    }
  }

  for (const candidate of candidates) {
    if (pathExists(path.join(candidate, 'cargo.exe'))) {
      return candidate;
    }
  }

  return null;
}

export function buildTauriEnvironment(env = process.env, options = {}) {
  const platform = options.platform ?? process.platform;
  const pathExists = options.pathExists ?? existsSync;
  const nextEnv = { ...env };

  const cargoBin = findLocalCargoBin(nextEnv, { platform, pathExists });
  if (!cargoBin) {
    return nextEnv;
  }

  const pathKey = getPathKey(nextEnv);
  const pathDelimiter = platform === 'win32' ? ';' : ':';
  const existingPath = nextEnv[pathKey] ?? '';
  const pathEntries = existingPath
    .split(pathDelimiter)
    .filter(Boolean);
  const normalizedCargoBin = normalizePathEntry(cargoBin, platform);
  const hasCargoBin = pathEntries.some(
    (entry) => normalizePathEntry(entry, platform) === normalizedCargoBin,
  );

  if (!hasCargoBin) {
    nextEnv[pathKey] = existingPath ? `${cargoBin}${pathDelimiter}${existingPath}` : cargoBin;
  }

  return nextEnv;
}

export function buildTauriInvocation(args, options = {}) {
  const cwd = normalizeWindowsWorkingDirectory(options.cwd ?? process.cwd());
  const normalizedPackageRoot = normalizeWindowsWorkingDirectory(options.packageRoot ?? packageRoot);
  const cliScript = path.join(normalizedPackageRoot, 'node_modules', '@tauri-apps', 'cli', 'tauri.js');
  const env = buildTauriEnvironment(options.env ?? process.env, {
    platform: options.platform,
    pathExists: options.pathExists,
  });

  return {
    cliScript,
    command: process.execPath,
    cwd,
    env,
    args: [cliScript, ...args],
  };
}

export async function runTauri(args, options = {}) {
  const invocation = buildTauriInvocation(args, options);

  if (!existsSync(invocation.cliScript)) {
    throw new Error(`Could not find Tauri CLI at ${invocation.cliScript}. Run npm install first.`);
  }

  const child = spawn(invocation.command, invocation.args, {
    cwd: invocation.cwd,
    env: invocation.env,
    stdio: 'inherit',
    windowsHide: false,
  });

  return await new Promise((resolve, reject) => {
    child.once('error', reject);
    child.once('exit', (code, signal) => resolve({ code, signal }));
  });
}

async function main() {
  const currentCwd = process.cwd();
  const normalizedCwd = normalizeWindowsWorkingDirectory(currentCwd);

  if (normalizedCwd !== currentCwd) {
    process.chdir(normalizedCwd);
  }

  const { code, signal } = await runTauri(process.argv.slice(2), {
    cwd: normalizedCwd,
  });

  if (signal) {
    process.kill(process.pid, signal);
  }

  process.exit(code ?? 0);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : error);
    process.exit(1);
  });
}
