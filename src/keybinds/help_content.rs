/// A logical group of help entries rendered together in the help overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpSection {
    pub title: &'static str,
    pub entries: Vec<HelpEntry>,
}

/// A single keybinding or key sequence shown in help.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpEntry {
    pub keys: &'static str,
    pub action: &'static str,
}

/// Returns the structured help content used by the help overlay.
pub fn help_sections() -> Vec<HelpSection> {
    vec![
        HelpSection {
            title: "Global",
            entries: vec![
                entry("H", "Toggle help"),
                entry(":q", "Quit"),
                entry(":q!", "Quit without saving output"),
                entry("Ctrl-C", "Force quit"),
                entry("W", "Toggle word wrap"),
                entry(
                    "count+nav",
                    "Repeat supported navigation including char search",
                ),
                entry("0 / 10j", "0 keeps line-start unless extending a count"),
                entry(
                    "f0 / t;",
                    "Char-search targets treat digits and punctuation literally",
                ),
                entry(
                    "; / ,",
                    "Repeat the last successful char search / reverse it",
                ),
                entry("/ / ?", "Search forward / backward"),
                entry("n / N", "Repeat search / reverse direction"),
                entry("4d / 5dd", "Counted mutation commands are unsupported"),
                entry("Tab", "Toggle annotation panel focus"),
                entry("Wheel ↑/↓", "Scroll the focused document/list/overlay"),
            ],
        },
        HelpSection {
            title: "Normal Mode",
            entries: vec![
                entry("h/j/k/l", "Move cursor"),
                entry("w/b/e", "Move by word"),
                entry("0/$", "Move to line start/end"),
                entry(
                    "f/F/t/T",
                    "Move to / before a character on the current line",
                ),
                entry(
                    "; / ,",
                    "Repeat the last successful char search / reverse it",
                ),
                entry("/", "Search forward"),
                entry("?", "Search backward"),
                entry("n", "Repeat search"),
                entry("N", "Repeat search in the opposite direction"),
                entry("gj/gk", "Move by screen line when content is wrapped"),
                entry("gg/G", "Move to document top/bottom"),
                entry("Ctrl-d/u", "Move half page down/up"),
                entry("Ctrl-f/b", "Move full page down/up"),
                entry("v", "Enter visual mode"),
                entry("V", "Enter visual line mode"),
                entry("i", "Create insertion annotation"),
                entry("gc", "Create global comment annotation"),
                entry("]a/[a", "Jump to next/previous annotation"),
                entry("Esc", "Hide annotation panel"),
                entry("Wheel ↑/↓", "Move like k / j"),
            ],
        },
        HelpSection {
            title: "Visual Mode",
            entries: vec![
                entry("h/j/k/l", "Extend selection"),
                entry("w/b/e", "Extend selection by word"),
                entry("0/$", "Extend selection to line start/end"),
                entry(
                    "f/F/t/T",
                    "Extend selection to / before a character on the current line",
                ),
                entry(
                    "; / ,",
                    "Repeat the last successful char search / reverse it",
                ),
                entry("/", "Search forward and extend selection to match"),
                entry("?", "Search backward and extend selection to match"),
                entry("n", "Repeat search and extend selection to match"),
                entry(
                    "N",
                    "Repeat search in the opposite direction and extend selection to match",
                ),
                entry("gj/gk", "Extend selection by screen line when wrapped"),
                entry("d", "Create deletion annotation"),
                entry("c", "Create comment annotation"),
                entry("r", "Create replacement annotation"),
                entry("Esc", "Cancel selection"),
                entry("Wheel ↑/↓", "Extend selection like k / j"),
            ],
        },
        HelpSection {
            title: "Visual Line Mode",
            entries: vec![
                entry("h/j/k/l", "Extend selection by line/motion"),
                entry("w/b/e", "Extend selection by line/motion"),
                entry("0/$", "Extend selection by line/motion"),
                entry("f/F/t/T", "Extend selection by line/motion"),
                entry("; / ,", "Extend selection by line/motion"),
                entry("/ / ?", "Extend selection by line/motion"),
                entry("n / N", "Extend selection by line/motion"),
                entry("d", "Create deletion annotation"),
                entry("c", "Create comment annotation"),
                entry("r", "Create replacement annotation"),
                entry("v", "Switch to charwise visual"),
                entry("V", "Exit visual line mode"),
                entry("Esc", "Cancel selection"),
                entry("Wheel ↑/↓", "Extend selection like k / j"),
            ],
        },
        HelpSection {
            title: "Insert Mode",
            entries: vec![
                entry("Ctrl-S", "Confirm input"),
                entry("Esc", "Cancel input"),
            ],
        },
        HelpSection {
            title: "Search Mode",
            entries: vec![
                entry("Enter", "Confirm search"),
                entry("Esc", "Cancel search"),
            ],
        },
        HelpSection {
            title: "Annotation List",
            entries: vec![
                entry("j/k", "Move selection"),
                entry("Space", "Inspect selected annotation"),
                entry("Up/Down", "Scroll inspect text"),
                entry("PgUp/PgDn", "Page inspect text"),
                entry("Ctrl-u/d", "Page inspect text"),
                entry("Enter", "Jump to selected annotation"),
                entry("Tab", "Unfocus annotation panel"),
                entry("dd", "Delete selected annotation"),
                entry("Esc", "Hide annotation panel"),
                entry("Wheel ↑/↓", "Move selection like k / j"),
            ],
        },
        HelpSection {
            title: "Command Mode",
            entries: vec![
                entry(":q", "Quit"),
                entry(":q!", "Force quit"),
                entry("Esc", "Cancel command"),
            ],
        },
    ]
}

fn entry(keys: &'static str, action: &'static str) -> HelpEntry {
    HelpEntry { keys, action }
}

#[cfg(test)]
mod tests {
    use super::{HelpEntry, HelpSection, help_sections};

    #[test]
    fn help_sections_match_required_groups() {
        let sections = help_sections();

        assert_eq!(
            sections
                .iter()
                .map(|section| section.title)
                .collect::<Vec<_>>(),
            vec![
                "Global",
                "Normal Mode",
                "Visual Mode",
                "Visual Line Mode",
                "Insert Mode",
                "Search Mode",
                "Annotation List",
                "Command Mode",
            ]
        );
    }

    #[test]
    fn global_section_covers_global_bindings() {
        let sections = help_sections();

        assert_eq!(
            sections[0],
            HelpSection {
                title: "Global",
                entries: vec![
                    HelpEntry {
                        keys: "H",
                        action: "Toggle help"
                    },
                    HelpEntry {
                        keys: ":q",
                        action: "Quit"
                    },
                    HelpEntry {
                        keys: ":q!",
                        action: "Quit without saving output"
                    },
                    HelpEntry {
                        keys: "Ctrl-C",
                        action: "Force quit"
                    },
                    HelpEntry {
                        keys: "W",
                        action: "Toggle word wrap"
                    },
                    HelpEntry {
                        keys: "count+nav",
                        action: "Repeat supported navigation including char search"
                    },
                    HelpEntry {
                        keys: "0 / 10j",
                        action: "0 keeps line-start unless extending a count"
                    },
                    HelpEntry {
                        keys: "f0 / t;",
                        action: "Char-search targets treat digits and punctuation literally"
                    },
                    HelpEntry {
                        keys: "; / ,",
                        action: "Repeat the last successful char search / reverse it"
                    },
                    HelpEntry {
                        keys: "/ / ?",
                        action: "Search forward / backward"
                    },
                    HelpEntry {
                        keys: "n / N",
                        action: "Repeat search / reverse direction"
                    },
                    HelpEntry {
                        keys: "4d / 5dd",
                        action: "Counted mutation commands are unsupported"
                    },
                    HelpEntry {
                        keys: "Tab",
                        action: "Toggle annotation panel focus"
                    },
                    HelpEntry {
                        keys: "Wheel ↑/↓",
                        action: "Scroll the focused document/list/overlay"
                    },
                ],
            }
        );
    }

    #[test]
    fn mode_specific_sections_cover_required_keys() {
        let sections = help_sections();

        assert!(contains_entry(&sections[1], "h/j/k/l", "Move cursor"));
        assert!(contains_entry(&sections[1], "w/b/e", "Move by word"));
        assert!(contains_entry(
            &sections[1],
            "0/$",
            "Move to line start/end"
        ));
        assert!(contains_entry(
            &sections[1],
            "f/F/t/T",
            "Move to / before a character on the current line"
        ));
        assert!(contains_entry(
            &sections[1],
            "; / ,",
            "Repeat the last successful char search / reverse it"
        ));
        assert!(contains_entry(&sections[1], "/", "Search forward"));
        assert!(contains_entry(&sections[1], "?", "Search backward"));
        assert!(contains_entry(&sections[1], "n", "Repeat search"));
        assert!(contains_entry(
            &sections[1],
            "N",
            "Repeat search in the opposite direction"
        ));
        assert!(contains_entry(
            &sections[1],
            "gj/gk",
            "Move by screen line when content is wrapped"
        ));
        assert!(contains_entry(
            &sections[1],
            "gg/G",
            "Move to document top/bottom"
        ));
        assert!(contains_entry(
            &sections[1],
            "Ctrl-d/u",
            "Move half page down/up"
        ));
        assert!(contains_entry(
            &sections[1],
            "Ctrl-f/b",
            "Move full page down/up"
        ));
        assert!(contains_entry(&sections[1], "v", "Enter visual mode"));
        assert!(contains_entry(&sections[1], "V", "Enter visual line mode"));
        assert!(contains_entry(
            &sections[1],
            "i",
            "Create insertion annotation"
        ));
        assert!(contains_entry(
            &sections[1],
            "gc",
            "Create global comment annotation"
        ));
        assert!(contains_entry(
            &sections[1],
            "]a/[a",
            "Jump to next/previous annotation"
        ));
        assert!(contains_entry(&sections[1], "Esc", "Hide annotation panel"));
        assert!(contains_entry(&sections[1], "Wheel ↑/↓", "Move like k / j"));

        assert!(contains_entry(
            &sections[2],
            "f/F/t/T",
            "Extend selection to / before a character on the current line"
        ));
        assert!(contains_entry(
            &sections[2],
            "; / ,",
            "Repeat the last successful char search / reverse it"
        ));
        assert!(contains_entry(
            &sections[2],
            "/",
            "Search forward and extend selection to match"
        ));
        assert!(contains_entry(
            &sections[2],
            "?",
            "Search backward and extend selection to match"
        ));
        assert!(contains_entry(
            &sections[2],
            "n",
            "Repeat search and extend selection to match"
        ));
        assert!(contains_entry(
            &sections[2],
            "N",
            "Repeat search in the opposite direction and extend selection to match"
        ));
        assert!(contains_entry(
            &sections[2],
            "gj/gk",
            "Extend selection by screen line when wrapped"
        ));
        assert!(contains_entry(
            &sections[2],
            "d",
            "Create deletion annotation"
        ));
        assert!(contains_entry(
            &sections[2],
            "c",
            "Create comment annotation"
        ));
        assert!(contains_entry(
            &sections[2],
            "r",
            "Create replacement annotation"
        ));
        assert!(contains_entry(&sections[2], "Esc", "Cancel selection"));
        assert!(contains_entry(
            &sections[2],
            "Wheel ↑/↓",
            "Extend selection like k / j"
        ));

        assert!(contains_entry(
            &sections[3],
            "h/j/k/l",
            "Extend selection by line/motion"
        ));
        assert!(contains_entry(
            &sections[3],
            "w/b/e",
            "Extend selection by line/motion"
        ));
        assert!(contains_entry(
            &sections[3],
            "0/$",
            "Extend selection by line/motion"
        ));
        assert!(contains_entry(
            &sections[3],
            "f/F/t/T",
            "Extend selection by line/motion"
        ));
        assert!(contains_entry(
            &sections[3],
            "; / ,",
            "Extend selection by line/motion"
        ));
        assert!(contains_entry(
            &sections[3],
            "/ / ?",
            "Extend selection by line/motion"
        ));
        assert!(contains_entry(
            &sections[3],
            "n / N",
            "Extend selection by line/motion"
        ));
        assert!(contains_entry(
            &sections[3],
            "d",
            "Create deletion annotation"
        ));
        assert!(contains_entry(
            &sections[3],
            "c",
            "Create comment annotation"
        ));
        assert!(contains_entry(
            &sections[3],
            "r",
            "Create replacement annotation"
        ));
        assert!(contains_entry(
            &sections[3],
            "v",
            "Switch to charwise visual"
        ));
        assert!(contains_entry(&sections[3], "V", "Exit visual line mode"));
        assert!(contains_entry(&sections[3], "Esc", "Cancel selection"));
        assert!(contains_entry(
            &sections[3],
            "Wheel ↑/↓",
            "Extend selection like k / j"
        ));

        assert!(contains_entry(&sections[4], "Ctrl-S", "Confirm input"));
        assert!(contains_entry(&sections[4], "Esc", "Cancel input"));

        assert!(contains_entry(&sections[5], "Enter", "Confirm search"));
        assert!(contains_entry(&sections[5], "Esc", "Cancel search"));

        assert!(contains_entry(&sections[6], "j/k", "Move selection"));
        assert!(contains_entry(
            &sections[6],
            "Space",
            "Inspect selected annotation"
        ));
        assert!(contains_entry(
            &sections[6],
            "Up/Down",
            "Scroll inspect text"
        ));
        assert!(contains_entry(
            &sections[6],
            "PgUp/PgDn",
            "Page inspect text"
        ));
        assert!(contains_entry(
            &sections[6],
            "Ctrl-u/d",
            "Page inspect text"
        ));
        assert!(contains_entry(
            &sections[6],
            "Enter",
            "Jump to selected annotation"
        ));
        assert!(contains_entry(
            &sections[6],
            "Tab",
            "Unfocus annotation panel"
        ));
        assert!(contains_entry(
            &sections[6],
            "dd",
            "Delete selected annotation"
        ));
        assert!(contains_entry(&sections[6], "Esc", "Hide annotation panel"));
        assert!(contains_entry(
            &sections[6],
            "Wheel ↑/↓",
            "Move selection like k / j"
        ));

        assert!(contains_entry(&sections[7], ":q", "Quit"));
        assert!(contains_entry(&sections[7], ":q!", "Force quit"));
        assert!(contains_entry(&sections[7], "Esc", "Cancel command"));
    }

    fn contains_entry(section: &HelpSection, keys: &str, action: &str) -> bool {
        section
            .entries
            .iter()
            .any(|entry| entry.keys == keys && entry.action == action)
    }
}
