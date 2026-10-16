use rstest::rstest;

use super::*;
use crate::test_util::fixture;

fn permission_screen() -> String {
    fixture!("cursor/2026.01.23/prompts/permission.txt").to_string()
}

fn question_screen() -> String {
    fixture!("cursor/2026.01.23/prompts/question.txt").to_string()
}

fn choices(ask: &Ask) -> Vec<(&str, String)> {
    ask.choices.iter().map(|c| (c.label.as_str(), c.keys.join(" "))).collect()
}

#[test]
fn a_command_prompt_offers_y_to_run_once_and_n_to_skip() {
    let ask = ask_screen(&permission_screen()).expect("a prompt");

    assert_eq!(
        (ask.question, ask.title.as_str(), ask.detail.as_str(), choices(&ask)),
        (
            false,
            "Run this command?",
            "$ git push origin main in ~/repo",
            vec![("allow", "y".to_string()), ("deny", "n".to_string())]
        )
    );
}

#[test]
fn the_allowlist_and_run_everything_rows_are_never_offered() {
    let ask = ask_screen(&permission_screen()).expect("a prompt");

    assert_eq!(ask.choices.len(), 2);
    assert!(ask.choices.iter().all(|c| c.keys != ["tab"] && c.keys != ["shift+tab"]));
}

#[rstest]
#[case::delete("Run this command?", "Delete this file?", "Run (once) (y)", "Delete (y)", "Skip (esc or n)", "Keep (n)")]
#[case::sandbox(
    "Run this command?",
    "Run this command outside the sandbox?",
    "Skip (esc or n)",
    "Skip (esc or n)",
    "x",
    "x"
)]
#[case::web_search("Run this command?", "Allow this web search?", "Run (once) (y)", "Allow search (y)", "x", "x")]
fn the_known_titles_and_rows_are_read(
    #[case] title: &str,
    #[case] other_title: &str,
    #[case] allow: &str,
    #[case] other_allow: &str,
    #[case] deny: &str,
    #[case] other_deny: &str,
) {
    let screen = permission_screen().replace(title, other_title).replace(allow, other_allow).replace(deny, other_deny);

    assert!(ask_screen(&screen).is_some());
}

#[test]
fn a_prompt_without_a_y_row_offers_deny_only_and_without_an_n_row_allow_only() {
    let no_allow = permission_screen().replace("Run (once) (y) (enter)", "Run (once) (enter)");
    let no_deny = permission_screen().replace("Skip (esc or n)", "Skip (esc)");

    assert_eq!(
        (choices(&ask_screen(&no_allow).expect("a prompt")), choices(&ask_screen(&no_deny).expect("a prompt"))),
        (vec![("deny", "n".to_string())], vec![("allow", "y".to_string())])
    );
}

#[rstest]
#[case::a_title_not_in_the_list("Run this command?", "Run this thing?")]
#[case::no_rule_above("──────────────────────────────────────────────────\n", "")]
#[case::neither_row("Run (once) (y) (enter)", "Run (once) (enter)")]
fn anything_else_gives_no_prompt(#[case] from: &str, #[case] to: &str) {
    let screen = permission_screen().replace(from, to).replace("Skip (esc or n)", "Skip");

    assert_eq!(ask_screen(&screen), None);
}

#[test]
fn text_after_the_prompt_gives_no_ask() {
    assert_eq!(ask_screen(&format!("{}\n some output\n", permission_screen())), None);
}

#[test]
fn a_changed_command_in_the_banner_changes_the_ask() {
    let changed = permission_screen().replace("git push origin main", "git push --force   ");

    assert_ne!(ask_screen(&changed), ask_screen(&permission_screen()));
}

#[test]
fn a_clarifying_question_is_answered_by_walking_to_the_top_and_down_to_the_row() {
    let ask = ask_screen(&question_screen()).expect("a question");

    assert_eq!(
        (ask.question, ask.title.as_str(), choices(&ask)),
        (
            true,
            "Which colour do you prefer?",
            vec![
                ("Red", "up up enter".to_string()),
                ("Green", "up up down enter".to_string()),
                ("Blue", "up up down down enter".to_string())
            ]
        )
    );
}

fn question_with(rows: &[&str]) -> String {
    let footer = "↑/↓ option · ←/→ question · Space/Enter select · Enter to submit · Esc to skip";
    let width = footer.chars().count() + 4;
    let top =
        format!("╭─ Clarifying Questions {}╮", "─".repeat(width - 1 - "╭─ Clarifying Questions ".chars().count()));
    let mut lines = vec![top, format!("│ {:<w$} │", "1. Which colour do you prefer?", w = width - 4)];
    lines.extend(rows.iter().map(|row| format!("│ {row:<w$} │", w = width - 4)));
    lines.push(format!("│ {footer} │"));
    lines.push(format!("╰{}╯", "─".repeat(width - 2)));
    lines.join("\n")
}

#[test]
fn an_other_row_counts_for_the_walk_but_is_never_offered() {
    let screen = question_with(&["› [ ] Red", "  [ ] Green", "  [ ] Other"]);

    let ask = ask_screen(&screen).expect("a question");

    assert_eq!(choices(&ask), [("Red", "up up enter".to_string()), ("Green", "up up down enter".to_string())]);
}

#[test]
fn a_selected_row_and_a_cursor_anywhere_are_fine_and_an_other_row_in_the_middle_is_not() {
    assert!(ask_screen(&question_with(&["  [ ] Red", "› [x] Green"])).is_some());
    assert_eq!(ask_screen(&question_with(&["  [ ] Red", "  [ ] Other", "  [ ] Blue"])), None);
}

#[rstest]
#[case::two_questions("1. Which colour do you prefer?", "1. Which colour?\n  [ ] x\n2. And a size?")]
#[case::multi_select("Which colour do you prefer?", "Which colour do you prefer? (multi-select)")]
#[case::another_title("Clarifying Questions", "Something Else")]
#[case::a_different_footer("Esc to skip", "Esc to quit")]
fn another_question_gives_no_ask(#[case] from: &str, #[case] to: &str) {
    assert_eq!(ask_screen(&question_screen().replace(from, to)), None);
}

#[test]
fn text_after_the_question_box_gives_no_ask() {
    assert_eq!(ask_screen(&format!("{}\n typing...\n", question_screen())), None);
}
