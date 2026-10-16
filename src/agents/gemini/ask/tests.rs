use std::fmt::Write;

use rstest::rstest;

use super::*;
use crate::test_util::fixture;

fn permission_screen() -> String {
    fixture!("gemini/0.64.0/prompts/permission.txt").to_string()
}

fn question_screen() -> String {
    fixture!("gemini/0.64.0/prompts/question.txt").to_string()
}

fn choices(ask: &Ask) -> Vec<(&str, &str)> {
    ask.choices.iter().map(|c| (c.label.as_str(), c.keys[0].as_str())).collect()
}

fn boxed(inner: &str) -> String {
    let width = 80;
    let mut out = vec![format!("╭{}╮", "─".repeat(width - 2))];
    out.extend(inner.lines().map(|line| format!("│ {line:<w$} │", w = width - 4)));
    out.push(format!("╰{}╯", "─".repeat(width - 2)));
    out.join("\n")
}

#[test]
fn a_tool_confirmation_offers_allow_once_and_the_escape_row_by_their_digits() {
    let ask = ask_screen(&permission_screen()).expect("a confirmation");

    assert_eq!(
        (ask.question, ask.title.as_str(), ask.detail.as_str(), choices(&ask)),
        (
            false,
            "Allow execution of [Shell]?",
            "echo \"Line 44\"\ncurl https://täst.com",
            vec![("allow", "1"), ("deny", "3")]
        )
    );
}

#[test]
fn the_rows_that_persist_a_choice_are_never_offered() {
    let ask = ask_screen(&permission_screen()).expect("a confirmation");

    assert!(ask.choices.iter().all(|c| !c.label.contains("session") && !c.label.contains("future")));
    assert_eq!(ask.choices.len(), 2);
}

#[test]
fn what_is_outside_the_command_changes_the_ask() {
    let queued = permission_screen().replace("3 of 3", "1 of 3");

    assert_ne!(ask_screen(&queued), ask_screen(&permission_screen()));
}

#[test]
fn a_command_that_overflows_its_box_offers_deny_only() {
    let elided = permission_screen().replace(
        "curl https://täst.com                                                   ",
        "... 48 hidden (Ctrl+O) ...                                               ",
    );

    let ask = ask_screen(&elided).expect("a confirmation");

    assert_eq!(choices(&ask), [("deny", "3")]);
}

#[rstest]
#[case::the_first_row_is_not_exactly_allow_once("Allow once ", "Yes, allow once")]
#[case::no_escape_row("No, suggest changes (esc)", "No, suggest changes    ")]
#[case::an_unknown_question("Allow execution of [Shell]?", "To run [Shell], allow access to the following?")]
#[case::no_header("? Shell  Executes", "  Shell  Executes")]
fn anything_else_gives_no_ask(#[case] from: &str, #[case] to: &str) {
    assert_eq!(ask_screen(&permission_screen().replace(from, to)), None);
}

#[test]
fn another_box_below_or_text_after_the_confirmation_gives_no_ask() {
    let below = format!("{}{}\n", permission_screen(), boxed("Something else"));
    let after = format!("{}Working...\n", permission_screen());

    assert_eq!((ask_screen(&below), ask_screen(&after)), (None, None));
}

fn with_rows(count: u32) -> String {
    let bottom = format!("╰{}╯\n", "─".repeat(78));
    let mut rows = String::new();
    for n in 4..=count {
        writeln!(rows, "│   {n}. Another {n:<w$} │", w = 61).expect("a string");
    }
    permission_screen().replace(&bottom, &format!("{rows}{bottom}"))
}

#[test]
fn nine_rows_are_fine_and_ten_give_no_ask() {
    assert_eq!((ask_screen(&with_rows(9)).is_some(), ask_screen(&with_rows(10))), (true, None));
}

#[test]
fn a_single_question_offers_a_digit_per_option_but_not_the_custom_row() {
    let ask = ask_screen(&question_screen()).expect("a question");

    assert_eq!(
        (ask.question, ask.title.as_str(), choices(&ask)),
        (true, "Which authentication method should we use?", vec![("OAuth 2.0", "1"), ("JWT tokens", "2")])
    );
}

#[test]
fn a_question_inside_the_tool_box_is_read_too() {
    let inside = boxed(question_screen().trim_end());

    let ask = ask_screen(&inside).expect("a question");

    assert_eq!(ask.choices.len(), 2);
}

#[test]
fn a_changed_description_changes_the_question() {
    let changed = question_screen().replace("Stateless, good for APIs", "Something else");

    assert_ne!(ask_screen(&changed), ask_screen(&question_screen()));
}

#[rstest]
#[case::a_tab_strip_above("Which authentication", "← □ Tests │ □ Docs │ ≡ Review →\nWhich authentication")]
#[case::multiple_selection("Enter a custom value", "[x] All of the above")]
#[case::the_editing_footer("Enter to select · ↑/↓ to navigate · Esc to cancel", "Enter to submit")]
#[case::text_after_the_footer("Esc to cancel", "Esc to cancel\nsomething")]
fn another_question_screen_gives_no_ask(#[case] from: &str, #[case] to: &str) {
    assert_eq!(ask_screen(&question_screen().replace(from, to)), None);
}
