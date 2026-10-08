import type { EngineInterface, Register } from 'claude-code'

import {
    buildReviewMessage,
    failureMessage,
    lastAssistantText,
    launcherArgs,
    parseCommandArgs,
    reviewTarget,
    type ReviewRequest,
} from './review'

// ----------------------------------------------------------------
// Support functions
// ----------------------------------------------------------------

/**
 * Runs the Herdr launcher, then sends the export back as the person's next
 * message. Runs after the command has answered, so the review tab can stay open
 * as long as the reviewer needs.
 */
async function runReview($: EngineInterface, request: ReviewRequest) {
    const target = reviewTarget(request)
    const child = $.process.spawn({
        argv: ['bash', `${$.plugin.root}/bin/anno-herdr-review`, ...launcherArgs(request)],
        input: request.content,
    })

    let stdout = ''
    let stderr = ''
    for await (const { stream, text } of child) {
        if (stream === 'stdout') stdout += text
        else stderr += text
    }
    const { code } = await child.result

    if (code !== 0 || !stdout.trim()) {
        $.ui.toast(failureMessage(code, stderr), { timeoutMs: 8000 })
        return
    }

    await $.prompt.submit({
        text: buildReviewMessage(target, stdout),
        asUser: true,
    })
    $.ui.toast('Imported anno review into the conversation.')
}

/**
 * Starts a review in the background, reporting launch failures as a toast.
 */
function startReview($: EngineInterface, request: ReviewRequest) {
    void runReview($, request).catch((error: unknown) => {
        const message = error instanceof Error ? error.message : String(error)
        $.ui.toast(`anno review failed: ${message}`, { timeoutMs: 8000 })
    })
}

// ----------------------------------------------------------------
// Plugin
// ----------------------------------------------------------------

export const register: Register = (on) => {
    on('session.start', async ($, e, next) => {
        await $.command.register({
            name: 'anno-review',
            description:
                'Review a file in anno (new Herdr tab), then send the annotations back',
            argumentHint: '<path> [--syntax <syntax>] [--title <title>]',
            immediate: true,
        })
        await $.command.register({
            name: 'anno-last',
            description:
                'Annotate the last assistant message in anno (new Herdr tab), then send the annotations back',
            immediate: true,
        })
        return next(e)
    })

    on('command.run', { command: 'anno-review' }, async ($, e) => {
        const parsed = parseCommandArgs(e.args)
        if (!parsed.ok) return { text: parsed.message }
        if (!(await $.fs.exists(parsed.request.path!))) {
            return { text: `Review file not found: ${parsed.request.path}` }
        }

        startReview($, parsed.request)
        return { text: `Opened ${parsed.request.path} in anno.` }
    })

    on('command.run', { command: 'anno-last' }, async ($, e) => {
        if (e.args.trim().length > 0) return { text: 'Usage: /anno-last' }

        const content = lastAssistantText(await $.session.messages())
        if (content === null) return { text: 'No assistant message found in session.' }

        startReview($, {
            content,
            fileName: 'last-message.md',
            title: 'Last Agent Message',
        })
        return { text: 'Opened the last assistant message in anno.' }
    })
}
