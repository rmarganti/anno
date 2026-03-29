#!/usr/bin/env bash
set -euo pipefail

PROMPT='Grab the next ready bead and complete its work. Your work is not considered completed until:
1. All validations pass
2. Anything that will be useful to future workers on this issue is stored in beads'\''s memory
3. Commit all changed files (including ones you didn'\''t specifically change) with a conventional commit message.'

while true; do
    ready=$(bd ready --json 2>/dev/null || echo "[]")

    if [ -z "$ready" ] || [ "$ready" = "[]" ]; then
        echo "No ready beads. Exiting."
        exit 0
    fi

    echo "=== Ready beads found, dispatching to OpenCode ==="
    opencode run --model openai/gpt-5.4 "$PROMPT"
    echo "=== Iteration complete ==="
done
