use rstest::rstest;

use super::*;

#[rstest]
#[case("claude", "Claude Code")]
#[case("codex", "Codex")]
#[case("gemini", "Gemini CLI")]
#[case("opencode", "opencode")]
#[case("grok", "Grok Build")]
#[case("cursor", "Cursor CLI")]
#[case::an_unknown_agent_keeps_its_name("aider", "aider")]
fn an_agent_has_a_display_name(#[case] agent: &str, #[case] name: &str) {
    assert_eq!(label(agent), name);
}
