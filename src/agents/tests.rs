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
fn every_agent_cornercase_knows_answers() {
    let answering: Vec<&str> = ["claude", "codex", "opencode", "gemini", "grok", "cursor", "aider"]
        .into_iter()
        .filter(|a| answers(a))
        .collect();

    assert_eq!(answering, ["claude", "codex", "opencode", "gemini", "grok", "cursor"]);
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

#[test]
fn no_agent_reads_another_agents_screen() {
    let screens = [
        ("claude", include_str!("../../tests/fixtures/claude/2.1.296/prompts/permission.txt")),
        ("claude", include_str!("../../tests/fixtures/claude/2.1.302/prompts/question.txt")),
        ("codex", include_str!("../../tests/fixtures/codex/0.163.0/prompts/permission.txt")),
        ("codex", include_str!("../../tests/fixtures/codex/0.163.0/prompts/question.txt")),
        ("opencode", include_str!("../../tests/fixtures/opencode/1.19.3/prompts/permission.txt")),
        ("opencode", include_str!("../../tests/fixtures/opencode/1.19.3/prompts/question.txt")),
        ("gemini", include_str!("../../tests/fixtures/gemini/0.64.0/prompts/permission.txt")),
        ("gemini", include_str!("../../tests/fixtures/gemini/0.64.0/prompts/question.txt")),
        ("grok", include_str!("../../tests/fixtures/grok/1.0.45/prompts/permission.txt")),
        ("cursor", include_str!("../../tests/fixtures/cursor/2026.01.23/prompts/permission.txt")),
        ("cursor", include_str!("../../tests/fixtures/cursor/2026.01.23/prompts/permission-2.txt")),
        ("cursor", include_str!("../../tests/fixtures/cursor/2026.01.23/prompts/question.txt")),
    ];
    let agents = ["claude", "codex", "opencode", "gemini", "grok", "cursor"];

    for (owner, screen) in screens {
        for agent in agents {
            assert_eq!(ask_screen(agent, screen).is_some(), agent == owner, "{agent} reading a {owner} screen");
        }
    }
}

#[test]
fn only_opencode_is_read_while_working_and_cursor_while_idle_or_working() {
    let read = |status: &str| -> Vec<&str> {
        ["claude", "codex", "opencode", "gemini", "grok", "cursor", "aider"]
            .into_iter()
            .filter(|agent| reads_screen(agent, status))
            .collect()
    };

    assert_eq!(
        (read("working"), read("idle"), reads_screen("claude", "waiting"), reads_screen("aider", "waiting")),
        (vec!["opencode", "cursor"], vec!["cursor"], true, false)
    );
}
