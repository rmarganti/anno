import { describe, expect, test } from 'claude-code/testing'

import {
    buildReviewMessage,
    failureMessage,
    lastAssistantText,
    launcherArgs,
    parseCommandArgs,
    tokenizeArgs,
} from '../hooks/review'

const EXPORT = `<annotations file="/tmp/last-message.md" total="1">
<annotation type="global_comment">
<comment>
Looks good.
</comment>
</annotation>
</annotations>`

describe('argument parsing', () => {
    test('splits quoted arguments', async () => {
        expect(tokenizeArgs(`docs/a.md --title "API review"`)).toEqual([
            'docs/a.md',
            '--title',
            'API review',
        ])
    })

    test('reads a path with syntax and title', async () => {
        expect(
            parseCommandArgs(`notes.txt --syntax rs --title 'My review'`)
        ).toEqual({
            ok: true,
            request: { path: 'notes.txt', syntax: 'rs', title: 'My review' },
        })
    })

    test('strips leading @ from a mentioned path', async () => {
        expect(parseCommandArgs('@README.md')).toEqual({
            ok: true,
            request: { path: 'README.md' },
        })
        expect(parseCommandArgs('@@docs/a.md')).toEqual({
            ok: true,
            request: { path: 'docs/a.md' },
        })
    })

    test('rejects a missing path, extra paths and unknown flags', async () => {
        expect(parseCommandArgs('').ok).toBe(false)
        expect(parseCommandArgs('@').ok).toBe(false)
        expect(parseCommandArgs('a.md b.md').ok).toBe(false)
        expect(parseCommandArgs('a.md --bogus')).toEqual({
            ok: false,
            message: 'Unknown flag: --bogus',
        })
        expect(parseCommandArgs('a.md --title')).toEqual({
            ok: false,
            message: 'Missing value for --title',
        })
    })
})

describe('launcher arguments', () => {
    test('passes a path after --', async () => {
        expect(launcherArgs({ path: '-odd.md', syntax: 'md' })).toEqual([
            '--syntax',
            'md',
            '--',
            '-odd.md',
        ])
    })

    test('streams content over stdin', async () => {
        expect(
            launcherArgs({
                content: 'hi',
                fileName: 'last-message.md',
                title: 'Last Agent Message',
            })
        ).toEqual([
            '--title',
            'Last Agent Message',
            '--stdin',
            '--name',
            'last-message.md',
        ])
    })
})

describe('messages', () => {
    test('finds the last assistant message with text', async () => {
        expect(
            lastAssistantText([
                { role: 'assistant', text: 'First answer', toolUses: [] },
                { role: 'user', text: 'Follow up', toolUses: [] },
                { role: 'assistant', text: 'Second answer', toolUses: [] },
                { role: 'assistant', text: '  ', toolUses: [] },
            ])
        ).toBe('Second answer')
        expect(
            lastAssistantText([{ role: 'user', text: 'Hi', toolUses: [] }])
        ).toBe(null)
    })

    test('wraps the export like the Pi extension', async () => {
        expect(buildReviewMessage('Last Agent Message', `${EXPORT}\n`)).toBe(
            `I completed an anno review for Last Agent Message.\n\nStructured anno export:\n\n${EXPORT}`
        )
    })

    test('explains cancellations and failures', async () => {
        expect(failureMessage(3, '')).toBe(
            'anno review cancelled (quit without exporting).'
        )
        expect(
            failureMessage(1, 'anno-herdr-review: not running inside a Herdr pane\n')
        ).toBe('anno review failed: not running inside a Herdr pane')
        expect(failureMessage(null, '')).toBe('anno review failed.')
    })
})
