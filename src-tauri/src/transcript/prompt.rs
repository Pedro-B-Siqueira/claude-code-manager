//! Tells prompts typed by the user apart from the machine-generated `user` lines Claude Code
//! also writes (task notifications, local command output, hook context, interruptions).

const MACHINE_TAGS: &[&str] = &[
    "<task-notification>",
    "<local-command-stdout>",
    "<local-command-stderr>",
    "<local-command-caveat>",
    "<system-reminder>",
    "<bash-stdout>",
    "<bash-stderr>",
    "<user-prompt-submit-hook>",
];

const INTERRUPTION_PREFIX: &str = "[Request interrupted";

#[derive(Debug, Clone, Copy, Default)]
pub struct PromptFlags<'a> {
    pub is_meta: bool,
    pub is_compact_summary: bool,
    pub is_sidechain: bool,
    /// `origin.kind` on recent versions (`human`, `task-notification`…); absent on older ones.
    pub origin_kind: Option<&'a str>,
}

pub fn human_prompt(flags: PromptFlags<'_>, text: &str) -> Option<String> {
    if flags.is_meta || flags.is_compact_summary || flags.is_sidechain {
        return None;
    }
    if flags.origin_kind.is_some_and(|kind| kind != "human") {
        return None;
    }
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.starts_with(INTERRUPTION_PREFIX) {
        return None;
    }
    if let Some(command) = slash_command(trimmed) {
        return Some(command);
    }
    if let Some(shell_input) = tag_content(trimmed, "bash-input") {
        return Some(format!("! {}", shell_input.trim()));
    }
    if MACHINE_TAGS.iter().any(|tag| trimmed.starts_with(tag)) {
        return None;
    }
    Some(trimmed.to_owned())
}

fn slash_command(text: &str) -> Option<String> {
    let name = tag_content(text, "command-name")?.trim();
    let arguments = tag_content(text, "command-args").map(str::trim).unwrap_or_default();
    let command = if arguments.is_empty() { name.to_owned() } else { format!("{name} {arguments}") };
    Some(command)
}

fn tag_content<'t>(text: &'t str, tag: &str) -> Option<&'t str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = text.find(&open)? + open.len();
    let end = text[start..].find(&close)? + start;
    Some(&text[start..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed() -> PromptFlags<'static> {
        PromptFlags { origin_kind: Some("human"), ..PromptFlags::default() }
    }

    #[test]
    fn keeps_plain_typed_prompts() {
        assert_eq!(human_prompt(typed(), "  Fix the login bug\n").as_deref(), Some("Fix the login bug"));
    }

    #[test]
    fn rejects_meta_compact_sidechain_and_non_human_origin() {
        assert!(human_prompt(PromptFlags { is_meta: true, ..typed() }, "x").is_none());
        assert!(human_prompt(PromptFlags { is_compact_summary: true, ..typed() }, "x").is_none());
        assert!(human_prompt(PromptFlags { is_sidechain: true, ..typed() }, "x").is_none());
        let notification = PromptFlags { origin_kind: Some("task-notification"), ..PromptFlags::default() };
        assert!(human_prompt(notification, "done").is_none());
    }

    #[test]
    fn rejects_machine_tags_on_older_versions_without_origin() {
        let legacy = PromptFlags::default();
        assert!(human_prompt(legacy, "<task-notification><task-id>1</task-id></task-notification>").is_none());
        assert!(human_prompt(legacy, "<local-command-stdout>ok</local-command-stdout>").is_none());
        assert!(human_prompt(legacy, "[Request interrupted by user]").is_none());
    }

    #[test]
    fn renders_slash_commands_with_arguments() {
        let text = "<command-message>review is running…</command-message>\n<command-name>/review</command-name>\n<command-args>PR 42</command-args>";
        assert_eq!(human_prompt(PromptFlags::default(), text).as_deref(), Some("/review PR 42"));
        let bare = "<command-name>/clear</command-name><command-args></command-args>";
        assert_eq!(human_prompt(PromptFlags::default(), bare).as_deref(), Some("/clear"));
    }

    #[test]
    fn renders_shell_escapes() {
        assert_eq!(
            human_prompt(PromptFlags::default(), "<bash-input>git status</bash-input>").as_deref(),
            Some("! git status")
        );
    }
}
