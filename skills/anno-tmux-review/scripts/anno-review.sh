#!/usr/bin/env bash
set -euo pipefail

usage() {
	cat <<EOF
Usage: anno-review.sh <file> [--syntax <syntax>] [--title <title>]

Launch anno in a tmux window for interactive review, block until the
reviewer exits, then print any annotations to stdout.

Arguments:
  file              Path to the file to review (required)

Options:
  --syntax <syntax> Syntax hint for highlighting (default: md)
  --title <title>   Title shown in anno's status bar (default: Reviewing: <basename>)
  -h, --help        Show this help message

Exit codes:
  0  Review completed (annotations may or may not be present in stdout)
  1  Missing dependencies or invalid arguments
EOF
}

# ── Defaults ──────────────────────────────────────────────────────────
SYNTAX="md"
TITLE=""
FILE=""

# ── Parse arguments ───────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
	case "$1" in
	--syntax)
		SYNTAX="$2"
		shift 2
		;;
	--title)
		TITLE="$2"
		shift 2
		;;
	-h | --help)
		usage
		exit 0
		;;
	-*)
		echo "Unknown option: $1" >&2
		usage >&2
		exit 1
		;;
	*)
		FILE="$1"
		shift
		;;
	esac
done

if [[ -z "$FILE" ]]; then
	echo "Error: file argument is required" >&2
	usage >&2
	exit 1
fi

if [[ ! -f "$FILE" ]]; then
	echo "Error: file not found: $FILE" >&2
	exit 1
fi

# ── Check dependencies ────────────────────────────────────────────────
if ! command -v anno &>/dev/null; then
	echo "Error: anno not found on PATH" >&2
	exit 1
fi

if [[ -z "${TMUX:-}" ]]; then
	echo "Error: not running inside a tmux session" >&2
	exit 1
fi

# ── Prepare temp files ────────────────────────────────────────────────
if command -v md5sum &>/dev/null; then
	CONTENT_HASH=$(md5sum "$FILE" | cut -d' ' -f1)
elif command -v md5 &>/dev/null; then
	CONTENT_HASH=$(md5 -q "$FILE")
else
	CONTENT_HASH=$(cksum "$FILE" | cut -d' ' -f1)
fi
EXT="${FILE##*.}"
[[ "$EXT" == "$FILE" ]] && EXT="txt"
REVIEW_FILE="/tmp/anno-review-${CONTENT_HASH}.${EXT}"
SESSION_ID=$(date +%s%N)
OUTPUT_FILE="/tmp/anno-out-${SESSION_ID}.txt"

cp "$FILE" "$REVIEW_FILE"

# ── Default title from basename ───────────────────────────────────────
if [[ -z "$TITLE" ]]; then
	TITLE="Reviewing: $(basename "$FILE")"
fi

# ── Launch anno in tmux and block ─────────────────────────────────────
ESCAPED_OUTPUT_FILE=$(printf '%q' "$OUTPUT_FILE")
ESCAPED_TITLE=$(printf '%q' "$TITLE")
ESCAPED_SYNTAX=$(printf '%q' "$SYNTAX")
ESCAPED_REVIEW_FILE=$(printf '%q' "$REVIEW_FILE")

tmux new-window -n 'anno review' \
	"anno --output-file ${ESCAPED_OUTPUT_FILE} --title ${ESCAPED_TITLE} --syntax ${ESCAPED_SYNTAX} ${ESCAPED_REVIEW_FILE}; tmux wait-for -S anno-${SESSION_ID}"
tmux wait-for "anno-${SESSION_ID}"

# ── Output results ────────────────────────────────────────────────────
if [[ -f "$OUTPUT_FILE" ]] && [[ -s "$OUTPUT_FILE" ]]; then
	cat "$OUTPUT_FILE"
fi

# ── Clean up ──────────────────────────────────────────────────────────
rm -f "$REVIEW_FILE" "$OUTPUT_FILE"
