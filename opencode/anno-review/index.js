import { COMMAND_NAME, COMMAND_VALUE, PLUGIN_ID, USAGE, createLaunchPlan, createTempArtifacts, ensureReviewTarget, isAnnoAvailable, isInteractiveTerminal, isSupportedRoute, launchAnno, parseCommandArgs, summarizeLaunchResult } from './lib.js';

async function runAnnoReview(api, rawArgs) {
  if (!isSupportedRoute(api.route.current)) {
    api.ui.toast({
      variant: 'error',
      message: `/${COMMAND_NAME} is only available from the home or session prompt.`,
    });
    return;
  }

  const cwd = api.state.path.directory;
  if (!cwd) {
    api.ui.toast({
      variant: 'error',
      message: 'Paths are still syncing. Try again in a moment.',
    });
    return;
  }

  if (!isInteractiveTerminal()) {
    api.ui.toast({
      variant: 'error',
      message: `/${COMMAND_NAME} requires an interactive terminal.`,
    });
    return;
  }

  const parsed = parseCommandArgs(rawArgs);
  if (!parsed.ok) {
    api.ui.toast({ variant: 'error', message: parsed.message });
    return;
  }

  if (!isAnnoAvailable()) {
    api.ui.toast({
      variant: 'error',
      message: 'anno is not available on PATH.',
    });
    return;
  }

  const plan = createLaunchPlan(parsed.request, cwd);
  if (!plan.ok) {
    api.ui.toast({ variant: 'error', message: plan.message });
    return;
  }

  const target = ensureReviewTarget(plan.request.path);
  if (!target.ok) {
    api.ui.toast({ variant: 'error', message: target.message });
    return;
  }

  const artifacts = createTempArtifacts();
  try {
    for (const note of plan.notes) {
      api.ui.toast({ variant: 'info', message: note });
    }

    const launch = launchAnno(api.renderer, plan.buildArgs(artifacts.outputPath));
    const summary = summarizeLaunchResult(launch, artifacts.outputPath, plan.request.path);
    api.ui.toast({
      variant: summary.ok ? 'success' : summary.cancelled ? 'info' : 'error',
      message: summary.message,
    });
  } finally {
    artifacts.cleanup();
  }
}

function promptForArgs(api) {
  api.ui.dialog.replace(() =>
    api.ui.DialogPrompt({
      title: 'Anno review',
      placeholder: USAGE,
      onConfirm: (value) => {
        api.ui.dialog.clear();
        void runAnnoReview(api, value);
      },
      onCancel: () => {
        api.ui.dialog.clear();
      },
    }),
  );
}

const plugin = {
  id: PLUGIN_ID,
  async tui(api) {
    api.command.register(() => [
      {
        title: 'Anno review',
        value: COMMAND_VALUE,
        description: `Open anno in the current terminal. Usage: ${USAGE}`,
        category: 'Review',
        slash: { name: COMMAND_NAME },
        onSelect: () => {
          promptForArgs(api);
        },
      },
    ]);
  },
};

export default plugin;
export { runAnnoReview };
