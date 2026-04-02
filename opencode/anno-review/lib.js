import { spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { basename, join, resolve } from 'node:path';
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

function isRecord(value) {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value);
}

function normalizeAnnotation(annotation, index) {
  if (!isRecord(annotation)) {
    throw new Error(`Annotation ${index + 1} must be an object.`);
  }

  const annotationType = annotation.type;
  if (typeof annotationType !== 'string' || annotationType.length === 0) {
    throw new Error(`Annotation ${index + 1} is missing a string type.`);
  }

  const normalized = { type: annotationType };

  if ('line' in annotation) {
    if (typeof annotation.line !== 'number' || !Number.isInteger(annotation.line) || annotation.line < 1) {
      throw new Error(`Annotation ${index + 1} has an invalid line.`);
    }
    normalized.line = annotation.line;
  }

  if ('lines' in annotation) {
    if (typeof annotation.lines !== 'string' || annotation.lines.length === 0) {
      throw new Error(`Annotation ${index + 1} has invalid lines.`);
    }
    normalized.lines = annotation.lines;
  }

  if ('selected_text' in annotation) {
    if (typeof annotation.selected_text !== 'string') {
      throw new Error(`Annotation ${index + 1} has invalid selected_text.`);
    }
    normalized.selected_text = annotation.selected_text;
  }

  if ('text' in annotation) {
    if (typeof annotation.text !== 'string') {
      throw new Error(`Annotation ${index + 1} has invalid text.`);
    }
    normalized.text = annotation.text;
  }

  return normalized;
}

export function parseAnnoExport(raw) {
  let parsed;
  try {
    parsed = JSON.parse(raw);
  } catch (error) {
    const detail = error instanceof Error ? error.message : 'Unknown JSON parse error';
    throw new Error(`anno produced invalid JSON: ${detail}`);
  }

  if (!isRecord(parsed)) {
    throw new Error('anno export must be a JSON object.');
  }

  if (typeof parsed.source !== 'string' || parsed.source.length === 0) {
    throw new Error('anno export is missing a string source field.');
  }

  if (typeof parsed.total !== 'number' || !Number.isInteger(parsed.total) || parsed.total < 0) {
    throw new Error('anno export is missing a valid total field.');
  }

  if (!Array.isArray(parsed.annotations)) {
    throw new Error('anno export is missing an annotations array.');
  }

  const annotations = parsed.annotations.map((annotation, index) => normalizeAnnotation(annotation, index));
  if (annotations.length !== parsed.total) {
    throw new Error(`anno export total ${parsed.total} did not match ${annotations.length} annotations.`);
  }

  return {
    source: parsed.source,
    total: parsed.total,
    annotations,
  };
}

export function loadAnnoExport(outputPath) {
  if (!existsSync(outputPath)) {
    throw new Error(`anno did not write an export file at ${outputPath}.`);
  }

  return parseAnnoExport(readFileSync(outputPath, 'utf8'));
}

function formatAnnotationLocation(annotation) {
  if (annotation.line) return `line ${annotation.line}`;
  if (annotation.lines) return `lines ${annotation.lines}`;
  return 'document-wide';
}

function countByType(annotations) {
  const counts = new Map();
  for (const annotation of annotations) {
    counts.set(annotation.type, (counts.get(annotation.type) ?? 0) + 1);
  }
  return [...counts.entries()].sort(([left], [right]) => left.localeCompare(right));
}

function humanizeType(annotationType) {
  return annotationType.replaceAll('_', ' ');
}

export function formatImportPrompt(reviewedPath, exportData, notes = []) {
  const fileName = basename(reviewedPath);
  const heading = `Completed anno review for ${fileName} (${reviewedPath}).`;
  const lines = [heading, ''];

  if (exportData.total === 0) {
    lines.push('No annotations were exported. The review completed without recorded comments.', '');
  } else {
    const plural = exportData.total === 1 ? 'annotation' : 'annotations';
    lines.push(`Captured ${exportData.total} ${plural} from source \`${exportData.source}\`.`, '');

    const counts = countByType(exportData.annotations);
    if (counts.length > 0) {
      lines.push('Type breakdown:');
      for (const [annotationType, count] of counts) {
        lines.push(`- ${count} ${humanizeType(annotationType)}`);
      }
      lines.push('');
    }

    lines.push('Locations:');
    for (const annotation of exportData.annotations) {
      lines.push(`- ${humanizeType(annotation.type)} at ${formatAnnotationLocation(annotation)}`);
    }
    lines.push('');
  }

  if (notes.length > 0) {
    lines.push('Notes:');
    for (const note of notes) {
      lines.push(`- ${note}`);
    }
    lines.push('');
  }

  lines.push('JSON export:', '```json', JSON.stringify(exportData, null, 2), '```');
  return lines.join('\n');
}

function unwrapClientBoolean(result) {
  if (typeof result === 'boolean') return result;
  if (isRecord(result)) {
    if (typeof result.data === 'boolean') return result.data;
    if (typeof result.body === 'boolean') return result.body;
  }
  return false;
}

export async function importReviewToSession(client, prompt) {
  try {
    const appended = unwrapClientBoolean(
      await client.tui.appendPrompt({
        body: { text: prompt },
      }),
    );

    if (!appended) {
      return {
        ok: false,
        appended: false,
        submitted: false,
        message: 'OpenCode could not append the anno review to the active prompt.',
      };
    }

    const submitted = unwrapClientBoolean(await client.tui.submitPrompt());
    if (!submitted) {
      return {
        ok: false,
        appended: true,
        submitted: false,
        message:
          'OpenCode appended the anno review to the prompt, but could not submit it automatically. Review the prompt and submit it manually.',
      };
    }

    return {
      ok: true,
      appended: true,
      submitted: true,
      message: 'Imported the anno review into the active OpenCode session.',
    };
  } catch (error) {
    return {
      ok: false,
      appended: false,
      submitted: false,
      message: error instanceof Error ? `Failed to import anno review into OpenCode: ${error.message}` : 'Failed to import anno review into OpenCode.',
    };
  }
}
