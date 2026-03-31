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
                entry("?", "Toggle help"),
                entry(":q", "Quit"),
                entry(":q!", "Quit without saving output"),
                entry("Ctrl-C", "Force quit"),
                entry("W", "Toggle word wrap"),
                entry("Tab", "Toggle annotation list"),
            ],
        },
        HelpSection {
            title: "Normal Mode",
            entries: vec![
                entry("h/j/k/l", "Move cursor"),
                entry("w/b/e", "Move by word"),
                entry("0/$", "Move to line start/end"),
                entry("gg/G", "Move to document top/bottom"),
                entry("Ctrl-d/u", "Move half page down/up"),
                entry("Ctrl-f/b", "Move full page down/up"),
                entry("v", "Enter visual mode"),
                entry("i", "Create insertion annotation"),
                entry("gc", "Create global comment annotation"),
                entry("]a/[a", "Jump to next/previous annotation"),
            ],
        },
        HelpSection {
            title: "Visual Mode",
            entries: vec![
                entry("h/j/k/l", "Extend selection"),
                entry("w/b/e", "Extend selection by word"),
                entry("0/$", "Extend selection to line start/end"),
                entry("d", "Create deletion annotation"),
                entry("c", "Create comment annotation"),
                entry("r", "Create replacement annotation"),
                entry("Esc", "Cancel selection"),
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
            title: "Annotation List",
            entries: vec![
                entry("j/k", "Move selection"),
                entry("Space", "Inspect selected annotation"),
                entry("Enter", "Jump to selected annotation"),
                entry("dd", "Delete selected annotation"),
                entry("Esc", "Exit annotation list"),
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
                "Insert Mode",
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
                        keys: "?",
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
                        keys: "Tab",
                        action: "Toggle annotation list"
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

        assert!(contains_entry(&sections[3], "Ctrl-S", "Confirm input"));
        assert!(contains_entry(&sections[3], "Esc", "Cancel input"));

        assert!(contains_entry(&sections[4], "j/k", "Move selection"));
        assert!(contains_entry(
            &sections[4],
            "Space",
            "Inspect selected annotation"
        ));
        assert!(contains_entry(
            &sections[4],
            "Enter",
            "Jump to selected annotation"
        ));
        assert!(contains_entry(
            &sections[4],
            "dd",
            "Delete selected annotation"
        ));
        assert!(contains_entry(&sections[4], "Esc", "Exit annotation list"));

        assert!(contains_entry(&sections[5], ":q", "Quit"));
        assert!(contains_entry(&sections[5], ":q!", "Force quit"));
        assert!(contains_entry(&sections[5], "Esc", "Cancel command"));
    }

    fn contains_entry(section: &HelpSection, keys: &str, action: &str) -> bool {
        section
            .entries
            .iter()
            .any(|entry| entry.keys == keys && entry.action == action)
    }
}
