import type { SessionMessage } from 'claude-code'

// ----------------------------------------------------------------
// Types
// ----------------------------------------------------------------

export type ReviewRequest = {
    path?: string
    content?: string
    fileName?: string
    syntax?: string
    title?: string
}

export type ParsedArgs =
    | { ok: true; request: ReviewRequest }
    | { ok: false; message: string }

// ----------------------------------------------------------------
// Constants
// ----------------------------------------------------------------

export const FILE_USAGE =
    'Usage: /anno-review <path> [--syntax <syntax>] [--title <title>]'

/** Exit code the launcher uses when the reviewer quit without exporting. */
export const EXIT_CANCELLED = 3

// ----------------------------------------------------------------
// Support functions
// ----------------------------------------------------------------

/**
 * Splits a command string into shell-like argument tokens.
 */
export function tokenizeArgs(input: string): string[] {
    const tokens: string[] = []
    let current = ''
    let quote: '"' | "'" | null = null
    let escaping = false

    for (const char of input) {
        if (escaping) {
            current += char
            escaping = false
            continue
        }

        if (char === '\\') {
            escaping = true
            continue
        }

        if (quote) {
            if (char === quote) {
                quote = null
            } else {
                current += char
            }
            continue
        }

        if (char === '"' || char === "'") {
            quote = char
            continue
        }

        if (/\s/.test(char)) {
            if (current) {
                tokens.push(current)
                current = ''
            }
            continue
        }

        current += char
    }

    if (escaping) current += '\\'
    if (quote) {
        throw new Error('Unterminated quoted argument')
    }
    if (current) tokens.push(current)
    return tokens
}

/**
 * Parses `/anno-review` arguments into a review request.
 */
export function parseCommandArgs(args: string): ParsedArgs {
    let tokens: string[]
    try {
        tokens = tokenizeArgs(args.trim())
    } catch (error) {
        return {
            ok: false,
            message:
                error instanceof Error
                    ? error.message
                    : 'Could not parse command arguments',
        }
    }

    const request: ReviewRequest = {}
    const positionals: string[] = []

    for (let i = 0; i < tokens.length; i += 1) {
        const token = tokens[i]!
        if (token === '--syntax' || token === '--title') {
            const value = tokens[i + 1]
            if (!value) {
                return { ok: false, message: `Missing value for ${token}` }
            }
            if (token === '--syntax') request.syntax = value
            if (token === '--title') request.title = value
            i += 1
            continue
        }
        if (token.startsWith('--')) {
            return { ok: false, message: `Unknown flag: ${token}` }
        }
        positionals.push(token)
    }

    if (positionals.length !== 1) {
        return { ok: false, message: FILE_USAGE }
    }

    // Claude Code's `@file` mention syntax is the natural way to name a file.
    const path = positionals[0]!.replace(/^@+/, '')
    if (!path) return { ok: false, message: FILE_USAGE }

    request.path = path
    return { ok: true, request }
}

/**
 * Builds the launcher's argument list for a review request.
 */
export function launcherArgs(request: ReviewRequest): string[] {
    const args: string[] = []
    if (request.title) args.push('--title', request.title)
    if (request.syntax) args.push('--syntax', request.syntax)

    if (request.content !== undefined) {
        args.push('--stdin', '--name', request.fileName || 'review.md')
    } else {
        args.push('--', request.path!)
    }

    return args
}

/**
 * Finds the most recent assistant message with text content.
 */
export function lastAssistantText(
    messages: readonly SessionMessage[]
): string | null {
    for (let i = messages.length - 1; i >= 0; i -= 1) {
        const message = messages[i]!
        if (message.role !== 'assistant') continue

        const text = message.text.trim()
        if (text.length > 0) return text
    }

    return null
}

/**
 * Names the reviewed target for conversation messages and notices.
 */
export function reviewTarget(request: ReviewRequest): string {
    if (request.title) return request.title
    if (request.path) return request.path.split('/').pop() || request.path
    return 'reviewed content'
}

/**
 * Formats exported review results as a conversation message.
 */
export function buildReviewMessage(target: string, reviewExport: string): string {
    return [
        `I completed an anno review for ${target}.`,
        'Structured anno export:',
        reviewExport.trim(),
    ].join('\n\n')
}

/**
 * Explains why a launcher run produced no review.
 */
export function failureMessage(code: number | null, stderr: string): string {
    if (code === EXIT_CANCELLED) {
        return 'anno review cancelled (quit without exporting).'
    }

    const detail = stderr
        .trim()
        .split('\n')
        .pop()
        ?.replace(/^anno-herdr-review: /, '')

    return detail
        ? `anno review failed: ${detail}`
        : `anno review failed${code === null ? '' : ` (exit code ${code})`}.`
}
