pub(crate) mod claude;
pub(crate) mod codex;
pub(crate) mod cursor;
pub(crate) mod gemini;
pub(crate) mod grok;
mod numbered;
pub(crate) mod opencode;

use crate::ask::Ask;

/// Which agents get answer buttons: those with a strict parser for their screen in `src/agents/<agent>/`.
pub(crate) fn answers(agent: &str) -> bool {
    matches!(agent, "claude" | "codex" | "opencode" | "gemini" | "grok" | "cursor")
}

/// opencode's status stays `working` while it waits for a permission or an answer, so its working panes
/// are read too. This is the only exception, and everything that depends on it asks here.
pub(crate) fn polled(agent: &str, status: &str) -> bool {
    agent == "opencode" && status == "working"
}

/// Whether the screen of a pane with this agent and status can hold a prompt worth reading.
pub(crate) fn reads_screen(agent: &str, status: &str) -> bool {
    answers(agent) && (status == "waiting" || polled(agent, status))
}

pub(crate) fn ask_screen(agent: &str, screen: &str) -> Option<Ask> {
    match agent {
        "claude" => claude::ask::ask_screen(screen),
        "codex" => codex::ask::ask_screen(screen),
        "opencode" => opencode::ask::ask_screen(screen),
        "gemini" => gemini::ask::ask_screen(screen),
        "grok" => grok::ask::ask_screen(screen),
        "cursor" => cursor::ask::ask_screen(screen),
        _ => None,
    }
}

/// Whether the screen is read again after a press, to back out of a stage the press may have led to.
pub(crate) fn checks_after_press(agent: &str) -> bool {
    agent == "opencode"
}

/// After a key was pressed: whether the screen shows a stage the press should not have led to, which
/// is backed out of with Escape (opencode's "Always allow" confirmation, its "Reject permission" box).
pub(crate) fn backs_out(agent: &str, screen: &str) -> bool {
    agent == "opencode" && opencode::ask::stage(screen)
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
