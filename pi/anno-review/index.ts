import { spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { basename, extname, join, resolve } from "node:path";
import { tmpdir } from "node:os";

import type { ExtensionAPI, ExtensionCommandContext, ExtensionContext } from "@mariozechner/pi-coding-agent";
import { Type } from "@sinclair/typebox";

const COMMAND_NAME = "anno-review";
const TOOL_NAME = "anno_review";
const TOOL_LABEL = "Anno Review";
const TOOL_DESCRIPTION =
	"Open anno in the current terminal for interactive review. Use only when the user explicitly wants a human-in-the-loop anno session and Pi has an interactive TUI.";

const REVIEW_TOOL_PARAMS = Type.Object({
	path: Type.Optional(Type.String({ description: "Path to a file to review, resolved relative to the current working directory." })),
	content: Type.Optional(Type.String({ description: "Generated content to review by writing it to a temporary file before launching anno." })),
	fileName: Type.Optional(
		Type.String({ description: "Optional filename to use for generated content so anno can infer syntax from the extension." }),
	),
	syntax: Type.Optional(Type.String({ description: "Optional anno --syntax override." })),
	title: Type.Optional(Type.String({ description: "Optional anno --title value shown in the anno status bar." })),
});

type ReviewRequest = {
	path?: string;
	content?: string;
	fileName?: string;
	syntax?: string;
	title?: string;
};

type ReviewAnnotation = {
	type: string;
	line?: number;
	lines?: string;
	selected_text?: string;
	text?: string;
};

type ReviewExport = {
	source: string;
	total: number;
	annotations: ReviewAnnotation[];
};

type ReviewDetails = {
	ok: boolean;
	cancelled: boolean;
	message: string;
	mode?: "path" | "content";
	reviewedPath?: string;
	title?: string;
	syntax?: string;
	exitCode?: number | null;
	signal?: string | null;
	error?: string;
	export?: ReviewExport;
};

type SpawnOutcome = {
	status: number | null;
	signal: string | null;
	error?: string;
};

function tokenizeArgs(input: string): string[] {
	const tokens: string[] = [];
	let current = "";
	let quote: '"' | "'" | null = null;
	let escaping = false;

	for (const char of input) {
		if (escaping) {
			current += char;
			escaping = false;
			continue;
		}

		if (char === "\\") {
			escaping = true;
			continue;
		}

		if (quote) {
			if (char === quote) {
				quote = null;
			} else {
				current += char;
			}
			continue;
		}

		if (char === '"' || char === "'") {
			quote = char;
			continue;
		}

		if (/\s/.test(char)) {
			if (current) {
				tokens.push(current);
				current = "";
			}
			continue;
		}

		current += char;
	}

	if (escaping) current += "\\";
	if (quote) {
		throw new Error("Unterminated quoted argument");
	}
	if (current) tokens.push(current);
	return tokens;
}

function parseCommandArgs(args: string): { ok: true; request: ReviewRequest } | { ok: false; message: string } {
	let tokens: string[];
	try {
		tokens = tokenizeArgs(args.trim());
	} catch (error) {
		return { ok: false, message: error instanceof Error ? error.message : "Could not parse command arguments" };
	}

	if (tokens.length === 0) {
		return {
			ok: false,
			message: `Usage: /${COMMAND_NAME} <path> [--syntax <syntax>] [--title <title>]`,
		};
	}

	const request: ReviewRequest = {};
	const positionals: string[] = [];

	for (let i = 0; i < tokens.length; i += 1) {
		const token = tokens[i];
		if (token === "--syntax" || token === "--title") {
			const value = tokens[i + 1];
			if (!value) {
				return { ok: false, message: `Missing value for ${token}` };
			}
			if (token === "--syntax") request.syntax = value;
			if (token === "--title") request.title = value;
			i += 1;
			continue;
		}
		if (token.startsWith("--")) {
			return { ok: false, message: `Unknown flag: ${token}` };
		}
		positionals.push(token);
	}

	if (positionals.length !== 1) {
		return {
			ok: false,
			message: `Usage: /${COMMAND_NAME} <path> [--syntax <syntax>] [--title <title>]`,
		};
	}

	request.path = positionals[0];
	return { ok: true, request };
}

function chooseGeneratedFileName(request: ReviewRequest): string {
	const candidate = request.fileName?.trim() || "review";
	if (extname(candidate)) return candidate;
	const syntax = request.syntax?.trim();
	if (syntax && /^[a-z0-9_+-]+$/i.test(syntax)) {
		return `${candidate}.${syntax.toLowerCase()}`;
	}
	return `${candidate}.txt`;
}

function resolveReviewPath(request: ReviewRequest, ctx: ExtensionContext): { path: string; mode: "path" | "content"; cleanupDir: string } {
	const cleanupDir = mkdtempSync(join(tmpdir(), "anno-review-"));

	if (request.content !== undefined) {
		const fileName = chooseGeneratedFileName(request);
		const tempPath = join(cleanupDir, fileName);
		writeFileSync(tempPath, request.content, "utf8");
		return { path: tempPath, mode: "content", cleanupDir };
	}

	const resolvedPath = resolve(ctx.cwd, request.path!);
	return { path: resolvedPath, mode: "path", cleanupDir };
}

function isAnnoAvailable(): boolean {
	const result = spawnSync(process.env.SHELL || "/bin/sh", ["-lc", "command -v anno >/dev/null 2>&1"], {
		stdio: "ignore",
		env: process.env,
	});
	return result.status === 0;
}

async function launchAnno(ctx: ExtensionContext, args: string[]): Promise<SpawnOutcome> {
	return ctx.ui.custom<SpawnOutcome>((tui, _theme, _kb, done) => {
		tui.stop();
		process.stdout.write("\x1b[2J\x1b[H");

		let outcome: SpawnOutcome;
		try {
			const result = spawnSync("anno", args, {
				stdio: "inherit",
				env: process.env,
			});
			outcome = {
				status: result.status,
				signal: result.signal,
				error: result.error?.message,
			};
		} finally {
			tui.start();
			tui.requestRender(true);
		}

		done(outcome!);
		return { render: () => [], invalidate: () => {} };
	});
}

function buildReviewMessage(details: ReviewDetails): string {
	const reviewExport = details.export;
	const target = details.title || (details.reviewedPath ? basename(details.reviewedPath) : "reviewed content");
	if (!reviewExport) {
		return `I completed an anno review for ${target}, but no exported annotations were produced.`;
	}

	const summary =
		reviewExport.total === 0
			? `I reviewed ${target} in anno and found no annotations.`
			: `I reviewed ${target} in anno and captured ${reviewExport.total} annotation${reviewExport.total === 1 ? "" : "s"}.`;

	return `${summary}\n\nStructured anno export:\n\n\`\`\`json\n${JSON.stringify(reviewExport, null, 2)}\n\`\`\``;
}

async function runReview(request: ReviewRequest, ctx: ExtensionContext): Promise<ReviewDetails> {
	if (!request.path && request.content === undefined) {
		return {
			ok: false,
			cancelled: false,
			message: "Provide either a file path or generated content to review.",
		};
	}

	if (request.path && request.content !== undefined) {
		return {
			ok: false,
			cancelled: false,
			message: "Provide either path or content, not both.",
		};
	}

	if (!ctx.hasUI) {
		return {
			ok: false,
			cancelled: false,
			message: "Direct anno handoff requires Pi to be running with an interactive TUI.",
		};
	}

	if (!isAnnoAvailable()) {
		return {
			ok: false,
			cancelled: false,
			message: "anno is not available on PATH, so direct terminal handoff cannot run.",
		};
	}

	const { path: reviewPath, mode, cleanupDir } = resolveReviewPath(request, ctx);
	const outputPath = join(cleanupDir, "annotations.json");

	try {
		if (mode === "path" && !existsSync(reviewPath)) {
			return {
				ok: false,
				cancelled: false,
				message: `Review file not found: ${reviewPath}`,
				mode,
				reviewedPath: reviewPath,
				title: request.title,
				syntax: request.syntax,
			};
		}

		const launchArgs = ["--export-format", "json", "--output-file", outputPath];
		if (request.title) launchArgs.push("--title", request.title);
		if (request.syntax) launchArgs.push("--syntax", request.syntax);
		launchArgs.push(reviewPath);

		const outcome = await launchAnno(ctx, launchArgs);
		if (outcome.error) {
			return {
				ok: false,
				cancelled: false,
				message: `Failed to launch anno: ${outcome.error}`,
				mode,
				reviewedPath: reviewPath,
				title: request.title,
				syntax: request.syntax,
				exitCode: outcome.status,
				signal: outcome.signal,
				error: outcome.error,
			};
		}

		if (!existsSync(outputPath)) {
			return {
				ok: false,
				cancelled: outcome.status === 0,
				message:
					outcome.status === 0
						? "anno exited without exporting annotations. This usually means the review was cancelled with :q!."
						: `anno exited before producing JSON output${outcome.status === null ? "" : ` (exit code ${outcome.status})`}.`,
				mode,
				reviewedPath: reviewPath,
				title: request.title,
				syntax: request.syntax,
				exitCode: outcome.status,
				signal: outcome.signal,
			};
		}

		let reviewExport: ReviewExport;
		try {
			reviewExport = JSON.parse(readFileSync(outputPath, "utf8")) as ReviewExport;
		} catch (error) {
			return {
				ok: false,
				cancelled: false,
				message: "anno produced invalid JSON output.",
				mode,
				reviewedPath: reviewPath,
				title: request.title,
				syntax: request.syntax,
				exitCode: outcome.status,
				signal: outcome.signal,
				error: error instanceof Error ? error.message : String(error),
			};
		}

		return {
			ok: true,
			cancelled: false,
			message:
				reviewExport.total === 0
					? `anno review completed with no annotations for ${basename(reviewPath)}.`
					: `anno review captured ${reviewExport.total} annotation${reviewExport.total === 1 ? "" : "s"} for ${basename(reviewPath)}.`,
			mode,
			reviewedPath: reviewPath,
			title: request.title,
			syntax: request.syntax,
			exitCode: outcome.status,
			signal: outcome.signal,
			export: reviewExport,
		};
	} finally {
		rmSync(cleanupDir, { recursive: true, force: true });
	}
}

function reviewDetailsToToolResult(details: ReviewDetails) {
	return {
		content: [{ type: "text" as const, text: details.ok ? buildReviewMessage(details) : `Error: ${details.message}` }],
		details,
	};
}

export default function annoReviewExtension(pi: ExtensionAPI) {
	pi.registerCommand(COMMAND_NAME, {
		description: "Open anno to review a file, then import the exported JSON annotations back into the session",
		handler: async (args, ctx: ExtensionCommandContext) => {
			const parsed = parseCommandArgs(args);
			if (!parsed.ok) {
				ctx.ui.notify(parsed.message, "error");
				return;
			}

			const details = await runReview(parsed.request, ctx);
			if (!details.ok) {
				ctx.ui.notify(details.message, details.cancelled ? "info" : "error");
				return;
			}

			const message = buildReviewMessage(details);
			if (ctx.isIdle()) {
				pi.sendUserMessage(message);
				ctx.ui.notify("Imported anno review into the conversation.", "success");
			} else {
				pi.sendUserMessage(message, { deliverAs: "followUp" });
				ctx.ui.notify("Anno review queued as a follow-up message.", "success");
			}
		},
	});

	pi.registerTool({
		name: TOOL_NAME,
		label: TOOL_LABEL,
		description: TOOL_DESCRIPTION,
		promptSnippet: "Open anno interactively for human review and return the exported JSON annotations.",
		promptGuidelines: [
			"Use this tool only when the user explicitly wants an interactive anno review in the terminal.",
			"Do not call this tool in non-interactive or background workflows.",
		],
		parameters: REVIEW_TOOL_PARAMS,
		async execute(_toolCallId, rawParams, _signal, _onUpdate, ctx) {
			const params = rawParams as ReviewRequest;
			const details = await runReview(params, ctx);
			return reviewDetailsToToolResult(details);
		},
	});
}

export { COMMAND_NAME, TOOL_DESCRIPTION, TOOL_LABEL, TOOL_NAME };
