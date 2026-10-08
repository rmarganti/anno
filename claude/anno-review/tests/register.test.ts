import { expect, test } from 'claude-code/testing'
import type { Engine } from 'claude-code/testing'
import type { On, PromptSubmitInput } from 'claude-code'

const EXPORT = `<annotations file="/tmp/last-message.md" total="1">
<annotation type="global_comment">
<comment>
Tighten the intro.
</comment>
</annotation>
</annotations>`

/**
 * Runs a slash command as if the person typed it.
 */
function runCommand($: Engine, command: string, args = '') {
    return $.command.run({
        command,
        args,
        origin: { kind: 'composer' },
        presentation: { isFullscreen: false, columns: 120 },
    })
}

/**
 * Starts a session with the engine's command registry stubbed beneath the plugin.
 */
async function startSession($: Engine, on: On) {
    on('command.register', async (_$, e) => ({ value: { command: e.name } }))
    on('session.start', async () => ({ cwd: '/repo' }))
    await $.session.start({ cwd: '/repo', surface: 'terminal', isInteractive: true })
}

test('/anno-last reviews the last assistant message and submits the export', async ($, on) => {
    let spawned: { argv: readonly string[]; input?: string } | undefined
    let submit: (input: PromptSubmitInput) => void = () => {}
    const submitted = new Promise<PromptSubmitInput>((resolve) => {
        submit = resolve
    })

    on('session.messages', async () => ({
        value: [
            { role: 'user', text: 'Explain the plan', toolUses: [] },
            { role: 'assistant', text: 'Here is the plan.', toolUses: [] },
        ],
    }))
    on('process.spawn', async function* (_$, e) {
        spawned = { argv: e.argv, input: e.input }
        yield { stream: 'stdout', text: EXPORT }
        return { value: { code: 0, signal: null } }
    })
    on('prompt.submit', async (_$, e) => {
        submit(e)
        return { text: e.text }
    })
    on('ui.toast', async () => ({ value: undefined }))

    await startSession($, on)
    const ran = await runCommand($, 'anno-last')
    expect(ran.text).toBe('Opened the last assistant message in anno.')

    const prompt = await submitted
    expect(prompt.origin).toEqual({
        kind: 'plugin',
        name: 'anno-review',
        asUser: true,
    })
    expect(prompt.text).toBe(
        `I completed an anno review for Last Agent Message.\n\nStructured anno export:\n\n${EXPORT}`
    )
    expect(spawned?.argv.slice(2)).toEqual([
        '--title',
        'Last Agent Message',
        '--stdin',
        '--name',
        'last-message.md',
    ])
    expect(spawned?.input).toBe('Here is the plan.')
})

test('/anno-review rejects missing arguments without launching anno', async ($, on) => {
    let launched = false
    on('process.spawn', async function* () {
        launched = true
        return { value: { code: 0, signal: null } }
    })

    await startSession($, on)
    const ran = await runCommand($, 'anno-review')

    expect(ran.text).toBe(
        'Usage: /anno-review <path> [--syntax <syntax>] [--title <title>]'
    )
    expect(launched).toBe(false)
})

test('/anno-review reports a missing file without launching anno', async ($, on) => {
    let launched = false
    on('process.spawn', async function* () {
        launched = true
        return { value: { code: 0, signal: null } }
    })
    on('fs.exists', async () => ({ value: false }))

    await startSession($, on)
    const ran = await runCommand($, 'anno-review', 'missing.md')

    expect(ran.text).toBe('Review file not found: missing.md')
    expect(launched).toBe(false)
})
