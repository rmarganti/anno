import test from 'node:test';
import assert from 'node:assert/strict';

import {
  USAGE,
  createLaunchPlan,
  formatImportPrompt,
  importReviewToSession,
  isSupportedRoute,
  parseAnnoExport,
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

test('parseAnnoExport validates expected schema', () => {
  const exportData = parseAnnoExport(
    JSON.stringify({
      source: 'README.md',
      total: 2,
      annotations: [
        { type: 'comment', line: 4, text: 'Clarify this section.' },
        { type: 'global_comment', text: 'Looks good overall.' },
      ],
    }),
  );

  assert.deepEqual(exportData, {
    source: 'README.md',
    total: 2,
    annotations: [
      { type: 'comment', line: 4, text: 'Clarify this section.' },
      { type: 'global_comment', text: 'Looks good overall.' },
    ],
  });
});

test('parseAnnoExport rejects mismatched totals', () => {
  assert.throws(
    () =>
      parseAnnoExport(
        JSON.stringify({
          source: 'README.md',
          total: 1,
          annotations: [
            { type: 'comment', line: 4, text: 'Clarify this section.' },
            { type: 'comment', line: 9, text: 'Second note.' },
          ],
        }),
      ),
    /did not match 2 annotations/,
  );
});

test('formatImportPrompt includes summaries, notes, and json export', () => {
  const prompt = formatImportPrompt(
    '/tmp/project/README.md',
    {
      source: 'README.md',
      total: 2,
      annotations: [
        { type: 'comment', line: 4, text: 'Clarify this section.' },
        { type: 'global_comment', text: 'Looks good overall.' },
      ],
    },
    ['This anno version does not support --title, so the review ran without a custom title.'],
  );

  assert.match(prompt, /Completed anno review for README\.md \(\/tmp\/project\/README\.md\)\./);
  assert.match(prompt, /Captured 2 annotations from source `README\.md`\./);
  assert.match(prompt, /- 1 comment/);
  assert.match(prompt, /- 1 global comment/);
  assert.match(prompt, /- comment at line 4/);
  assert.match(prompt, /- global comment at document-wide/);
  assert.match(prompt, /Notes:/);
  assert.match(prompt, /```json/);
});

test('formatImportPrompt handles no-annotation reviews', () => {
  const prompt = formatImportPrompt('/tmp/project/README.md', {
    source: 'README.md',
    total: 0,
    annotations: [],
  });

  assert.match(prompt, /No annotations were exported/);
  assert.doesNotMatch(prompt, /Type breakdown:/);
});

test('importReviewToSession appends and submits prompt text', async () => {
  const calls = [];
  const client = {
    tui: {
      async appendPrompt(input) {
        calls.push(['appendPrompt', input]);
        return true;
      },
      async submitPrompt() {
        calls.push(['submitPrompt']);
        return { data: true };
      },
    },
  };

  const result = await importReviewToSession(client, 'review payload');
  assert.deepEqual(result, {
    ok: true,
    appended: true,
    submitted: true,
    message: 'Imported the anno review into the active OpenCode session.',
  });
  assert.deepEqual(calls, [
    ['appendPrompt', { body: { text: 'review payload' } }],
    ['submitPrompt'],
  ]);
});

test('importReviewToSession reports manual submit fallback when submit fails', async () => {
  const client = {
    tui: {
      async appendPrompt() {
        return true;
      },
      async submitPrompt() {
        return false;
      },
    },
  };

  const result = await importReviewToSession(client, 'review payload');
  assert.deepEqual(result, {
    ok: false,
    appended: true,
    submitted: false,
    message:
      'OpenCode appended the anno review to the prompt, but could not submit it automatically. Review the prompt and submit it manually.',
  });
});
