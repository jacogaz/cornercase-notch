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

#[test]
fn only_agents_with_a_tested_screen_parser_answer() {
    let answering: Vec<&str> = ["claude", "codex", "gemini", "opencode", "grok", "cursor", "aider"]
        .into_iter()
        .filter(|a| answers(a))
        .collect();

    assert_eq!(answering, ["claude", "codex"]);
}

#[test]
fn a_screen_goes_to_its_agent_parser() {
    let codex = include_str!("../../tests/fixtures/codex/0.163.0/prompts/permission.txt");
    let claude = include_str!("../../tests/fixtures/claude/2.1.296/prompts/permission.txt");

    assert_eq!(
        [("codex", codex), ("claude", claude), ("claude", codex), ("codex", claude), ("gemini", claude)]
            .map(|(agent, screen)| ask_screen(agent, screen).is_some()),
        [true, true, false, false, false]
    );
}
