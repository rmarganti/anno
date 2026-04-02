import { spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, rmSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { tmpdir } from 'node:os';

export const PLUGIN_ID = 'anno.opencode-review';
export const COMMAND_NAME = 'anno-review';
export const COMMAND_VALUE = 'anno-review.run';
export const USAGE = `/${COMMAND_NAME} <path> [--syntax <syntax>] [--title <title>]`;

export function tokenizeArgs(input) {
  const tokens = [];
  let current = '';
  let quote = null;
  let escaping = false;

  for (const char of input) {
    if (escaping) {
      current += char;
      escaping = false;
      continue;
    }

    if (char === '\\') {
      escaping = true;
      continue;
    }

    if (quote) {
      if (char === quote) quote = null;
      else current += char;
      continue;
    }

    if (char === '"' || char === "'") {
      quote = char;
      continue;
    }

    if (/\s/.test(char)) {
      if (current) {
        tokens.push(current);
        current = '';
      }
      continue;
    }

    current += char;
  }

  if (escaping) current += '\\';
  if (quote) throw new Error('Unterminated quoted argument');
  if (current) tokens.push(current);
  return tokens;
}

export function parseCommandArgs(input) {
  let tokens;
  try {
    tokens = tokenizeArgs(input.trim());
  } catch (error) {
    return {
      ok: false,
      message: error instanceof Error ? error.message : 'Could not parse command arguments',
    };
  }

  if (tokens.length === 0) {
    return { ok: false, message: `Usage: ${USAGE}` };
  }

  const positionals = [];
  const request = {};

  for (let index = 0; index < tokens.length; index += 1) {
    const token = tokens[index];
    if (token === '--syntax' || token === '--title') {
      const value = tokens[index + 1];
      if (!value) return { ok: false, message: `Missing value for ${token}` };
      if (token === '--syntax') request.syntax = value;
      if (token === '--title') request.title = value;
      index += 1;
      continue;
    }
    if (token.startsWith('--')) {
      return { ok: false, message: `Unknown flag: ${token}` };
    }
    positionals.push(token);
  }

  if (positionals.length !== 1) {
    return { ok: false, message: `Usage: ${USAGE}` };
  }

  request.path = positionals[0];
  return { ok: true, request };
}

export function isSupportedRoute(route) {
  return route?.name === 'home' || route?.name === 'session';
}

export function resolveReviewPath(path, cwd) {
  return resolve(cwd, path);
}

export function ensureReviewTarget(path) {
  if (!existsSync(path)) {
    return { ok: false, message: `Review file not found: ${path}` };
  }
  return { ok: true };
}

export function createTempArtifacts() {
  const dir = mkdtempSync(join(tmpdir(), 'anno-review-'));
  return {
    dir,
    outputPath: join(dir, 'annotations.json'),
    cleanup() {
      rmSync(dir, { recursive: true, force: true });
    },
  };
}

export function isInteractiveTerminal() {
  return Boolean(process.stdin.isTTY && process.stdout.isTTY);
}

let cachedHelp;

function getAnnoHelp() {
  if (cachedHelp) return cachedHelp;
  const result = spawnSync('anno', ['--help'], { encoding: 'utf8' });
  cachedHelp = result;
  return result;
}

export function resetCachedAnnoHelp() {
  cachedHelp = undefined;
}

export function isAnnoAvailable() {
  const result = getAnnoHelp();
  return !result.error && result.status === 0;
}

export function supportsAnnoFlag(flag) {
  const result = getAnnoHelp();
  if (result.error || result.status !== 0) return false;
  return result.stdout.includes(flag) || result.stderr.includes(flag);
}

export function clearRendererBuffer(renderer) {
  renderer?.currentRenderBuffer?.clear?.();
}

export function launchAnno(renderer, argv) {
  renderer.suspend();
  clearRendererBuffer(renderer);
  try {
    const result = spawnSync('anno', argv, {
      stdio: 'inherit',
      env: process.env,
    });
    return result;
  } finally {
    clearRendererBuffer(renderer);
    renderer.resume();
    renderer.requestRender();
  }
}

export function createLaunchPlan(request, cwd, support = supportsAnnoFlag) {
  const argv = ['--export-format', 'json', '--output-file'];
  const notes = [];

  if (!support('--export-format') || !support('--output-file')) {
    return {
      ok: false,
      message:
        'anno must support --export-format json and --output-file for this OpenCode integration. Upgrade anno and try again.',
    };
  }

  if (request.syntax) {
    if (!support('--syntax')) {
      return {
        ok: false,
        message: 'This anno version does not support --syntax. Upgrade anno and try again.',
      };
    }
  }

  let effectiveTitle = request.title;
  if (request.title && !support('--title')) {
    effectiveTitle = undefined;
    notes.push('This anno version does not support --title, so the review ran without a custom title.');
  }

  return {
    ok: true,
    request: {
      path: resolveReviewPath(request.path, cwd),
      syntax: request.syntax,
      title: effectiveTitle,
    },
    notes,
    buildArgs(outputPath) {
      const args = [...argv, outputPath];
      if (effectiveTitle) args.push('--title', effectiveTitle);
      if (request.syntax) args.push('--syntax', request.syntax);
      args.push(resolveReviewPath(request.path, cwd));
      return args;
    },
  };
}

export function summarizeLaunchResult(result, outputPath, reviewedPath) {
  if (result.error) {
    const detail = result.error.code === 'ENOENT' ? 'anno is not available on PATH.' : `Failed to launch anno: ${result.error.message}`;
    return { ok: false, cancelled: false, message: detail };
  }

  if (result.status !== 0) {
    const suffix = result.status == null ? '' : ` (exit code ${result.status})`;
    return {
      ok: false,
      cancelled: false,
      message: `anno exited before completing the review${suffix}.`,
    };
  }

  if (!existsSync(outputPath)) {
    return {
      ok: false,
      cancelled: true,
      message: `anno closed without exporting annotations for ${reviewedPath}. This usually means the review was cancelled.`,
    };
  }

  return {
    ok: true,
    cancelled: false,
    message: `anno review completed for ${reviewedPath}.`,
  };
}
