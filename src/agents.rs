pub(crate) mod claude;

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
