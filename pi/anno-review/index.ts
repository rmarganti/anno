import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";

const COMMAND_NAME = "anno-review";
const PLANNED_TOOL_NAME = "anno_review";

export default function annoReviewExtension(pi: ExtensionAPI) {
	pi.registerCommand(COMMAND_NAME, {
		description: "Review a file or generated content in anno (package scaffold; implementation pending)",
		handler: async (_args, ctx) => {
			if (ctx.hasUI) {
				ctx.ui.notify(
					`The ${COMMAND_NAME} extension is installed. Interactive handoff is planned here; see pi/anno-review/README.md for package layout and command/tool naming (${COMMAND_NAME}, ${PLANNED_TOOL_NAME}).`,
					"info",
				);
			}
		},
	});
}

export { COMMAND_NAME, PLANNED_TOOL_NAME };
