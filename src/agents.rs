pub(crate) mod claude;
pub(crate) mod codex;
mod numbered;

use crate::ask::Ask;

pub(crate) fn answers(agent: &str) -> bool {
    matches!(agent, "claude" | "codex")
}

pub(crate) fn ask_screen(agent: &str, screen: &str) -> Option<Ask> {
    match agent {
        "claude" => claude::ask::ask_screen(screen),
        "codex" => codex::ask::ask_screen(screen),
        _ => None,
    }
}

pub(crate) fn label(agent: &str) -> String {
    match agent {
        "claude" => "Claude Code",
        "codex" => "Codex",
        "gemini" => "Gemini CLI",
        "opencode" => "opencode",
        "grok" => "Grok Build",
        "cursor" => "Cursor CLI",
        other => other,
    }
    .to_string()
}

#[cfg(test)]
mod tests;
