import test from 'node:test';
import assert from 'node:assert/strict';

import plugin from './index.js';
import { COMMAND_NAME, COMMAND_VALUE, PLUGIN_ID, USAGE } from './lib.js';

test('plugin registers the anno review slash command', async () => {
  let registered;
  const api = {
    command: {
      register(factory) {
        registered = factory();
      },
    },
  };

  await plugin.tui(api);

  assert.equal(plugin.id, PLUGIN_ID);
  assert.equal(registered.length, 1);
  assert.deepEqual(registered[0], {
    title: 'Anno review',
    value: COMMAND_VALUE,
    description: `Open anno in the current terminal. Usage: ${USAGE}`,
    category: 'Review',
    slash: { name: COMMAND_NAME },
    onSelect: registered[0].onSelect,
  });
  assert.equal(typeof registered[0].onSelect, 'function');
});

test('slash command selection opens a dialog prompt with usage placeholder', async () => {
  let registered;
  let replacedDialog;
  let cleared = false;

  const api = {
    command: {
      register(factory) {
        registered = factory();
      },
    },
    ui: {
      dialog: {
        replace(factory) {
          replacedDialog = factory();
        },
        clear() {
          cleared = true;
        },
      },
      DialogPrompt(options) {
        return { kind: 'DialogPrompt', options };
      },
    },
  };

  await plugin.tui(api);
  registered[0].onSelect();

  assert.deepEqual(replacedDialog, {
    kind: 'DialogPrompt',
    options: {
      title: 'Anno review',
      placeholder: USAGE,
      onConfirm: replacedDialog.options.onConfirm,
      onCancel: replacedDialog.options.onCancel,
    },
  });
  assert.equal(typeof replacedDialog.options.onConfirm, 'function');
  assert.equal(typeof replacedDialog.options.onCancel, 'function');

  replacedDialog.options.onCancel();
  assert.equal(cleared, true);
});
