import test from 'node:test';
import assert from 'node:assert/strict';

import {
  USAGE,
  createLaunchPlan,
  isSupportedRoute,
  parseCommandArgs,
  resolveReviewPath,
  summarizeLaunchResult,
  tokenizeArgs,
} from './lib.js';

test('tokenizeArgs supports quotes and escapes', () => {
  assert.deepEqual(tokenizeArgs('"docs/My File.md" --title "API Review" path\\ with\\ spaces'), [
    'docs/My File.md',
    '--title',
    'API Review',
    'path with spaces',
  ]);
});

test('parseCommandArgs parses path syntax and title', () => {
  const result = parseCommandArgs('"docs/My File.md" --syntax markdown --title "API Review"');
  assert.equal(result.ok, true);
  assert.deepEqual(result.request, {
    path: 'docs/My File.md',
    syntax: 'markdown',
    title: 'API Review',
  });
});

test('parseCommandArgs rejects missing path', () => {
  const result = parseCommandArgs('--title Review');
  assert.deepEqual(result, {
    ok: false,
    message: `Usage: ${USAGE}`,
  });
});

test('parseCommandArgs rejects unknown flags', () => {
  const result = parseCommandArgs('README.md --bogus nope');
  assert.deepEqual(result, {
    ok: false,
    message: 'Unknown flag: --bogus',
  });
});

test('isSupportedRoute accepts home and session only', () => {
  assert.equal(isSupportedRoute({ name: 'home' }), true);
  assert.equal(isSupportedRoute({ name: 'session', params: { sessionID: '123' } }), true);
  assert.equal(isSupportedRoute({ name: 'plugin' }), false);
});

test('resolveReviewPath resolves relative paths against cwd', () => {
  assert.equal(resolveReviewPath('README.md', '/tmp/project'), '/tmp/project/README.md');
});

test('createLaunchPlan drops unsupported title flag but keeps syntax', () => {
  const support = (flag) => flag !== '--title';
  const result = createLaunchPlan(
    { path: 'README.md', syntax: 'markdown', title: 'Review' },
    '/tmp/project',
    support,
  );

  assert.equal(result.ok, true);
  assert.deepEqual(result.notes, [
    'This anno version does not support --title, so the review ran without a custom title.',
  ]);
  assert.deepEqual(result.buildArgs('/tmp/out.json'), [
    '--export-format',
    'json',
    '--output-file',
    '/tmp/out.json',
    '--syntax',
    'markdown',
    '/tmp/project/README.md',
  ]);
});

test('createLaunchPlan requires JSON export flags', () => {
  const result = createLaunchPlan({ path: 'README.md' }, '/tmp/project', () => false);
  assert.deepEqual(result, {
    ok: false,
    message:
      'anno must support --export-format json and --output-file for this OpenCode integration. Upgrade anno and try again.',
  });
});

test('summarizeLaunchResult treats missing export as cancellation', () => {
  const result = summarizeLaunchResult({ status: 0 }, '/definitely/missing/output.json', '/tmp/project/README.md');
  assert.deepEqual(result, {
    ok: false,
    cancelled: true,
    message:
      'anno closed without exporting annotations for /tmp/project/README.md. This usually means the review was cancelled.',
  });
});
