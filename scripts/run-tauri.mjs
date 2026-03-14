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

export function buildTauriInvocation(args, options = {}) {
  const cwd = normalizeWindowsWorkingDirectory(options.cwd ?? process.cwd());
  const normalizedPackageRoot = normalizeWindowsWorkingDirectory(options.packageRoot ?? packageRoot);
  const cliScript = path.join(normalizedPackageRoot, 'node_modules', '@tauri-apps', 'cli', 'tauri.js');

  return {
    cliScript,
    command: process.execPath,
    cwd,
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
    env: process.env,
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
