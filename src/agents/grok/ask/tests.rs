use rstest::rstest;

use super::*;
use crate::test_util::fixture;

fn permission_screen() -> String {
    fixture!("grok/1.0.45/prompts/permission.txt").to_string()
}

fn question_screen() -> String {
    fixture!("grok/1.0.45/prompts/question.txt").to_string()
}

fn choices(ask: &Ask) -> Vec<(&str, &str)> {
    ask.choices.iter().map(|c| (c.label.as_str(), c.keys[0].as_str())).collect()
}

#[test]
fn a_permission_offers_the_digits_of_yes_proceed_and_no_reject() {
    let ask = ask_screen(&permission_screen()).expect("a permission");

    assert_eq!(
        (ask.question, ask.title.as_str(), ask.detail.as_str(), choices(&ask)),
        (false, "Allow `cargo`?", "cargo test --workspace", vec![("allow", "3"), ("deny", "4")])
    );
}

#[test]
fn the_always_rows_are_never_offered_and_the_cursor_row_does_not_matter() {
    let on_always = permission_screen()
        .replace("3 (●) Yes, proceed", "3 (○) Yes, proceed")
        .replace("1 (○) Yes, and", "1 (●) Yes, and");

    let ask = ask_screen(&on_always).expect("a permission");

    assert_eq!(choices(&ask), [("allow", "3"), ("deny", "4")]);
}

#[test]
fn an_edit_allows_with_a_bare_yes() {
    let edit = permission_screen().replace("Yes, proceed", "Yes");

    assert_eq!(choices(&ask_screen(&edit).expect("a permission")), [("allow", "3"), ("deny", "4")]);
}

#[test]
fn a_command_cut_short_by_the_card_offers_deny_only() {
    let elided =
        permission_screen().replace("▌  cargo test --workspace", "▌  cargo test --workspace\n▌  ... Ctrl-F to expand");

    assert_eq!(choices(&ask_screen(&elided).expect("a permission")), [("deny", "4")]);
}

#[rstest]
#[case::no_reject_row("No, reject (type to add feedback)", "No, reject")]
#[case::only_always_rows("3 (●) Yes, proceed", "3 (●) Yes, and don't ask again")]
#[case::a_different_title("Allow `cargo`?", "Run `cargo`")]
#[case::a_skipped_number("4 (○) No", "5 (○) No")]
#[case::two_cursors("2 (○) Always allow", "2 (●) Always allow")]
fn anything_else_gives_no_permission(#[case] from: &str, #[case] to: &str) {
    assert_eq!(ask_screen(&permission_screen().replace(from, to)), None);
}

#[test]
fn text_after_the_card_gives_no_ask() {
    let after = format!("{}\n ctrl+o yolo · shift+tab mode\n", permission_screen());

    assert_eq!(ask_screen(&after), None);
}

#[test]
fn a_changed_command_changes_the_ask() {
    let changed = permission_screen().replace("cargo test --workspace\n▌", "cargo test --all\n▌");

    assert_ne!(ask_screen(&changed), ask_screen(&permission_screen()));
}

#[rstest]
#[case::as_it_is("1 (●) Red", "1 (●) Red")]
#[case::another_row_under_the_cursor("1 (●) Red", "1 (○) Red")]
fn a_question_gives_no_ask_since_a_digit_may_be_typed_text(#[case] from: &str, #[case] to: &str) {
    assert_eq!(ask_screen(&question_screen().replace(from, to)), None);
}

#[test]
fn the_title_must_be_the_first_line_of_the_card_and_no_other_line_may_look_like_one() {
    let late = permission_screen().replace(
        "▌  Allow `cargo`?\n▌  cargo test --workspace",
        "▌  Something\n▌  Allow `cargo`?\n▌  cargo test --workspace",
    );
    let second = permission_screen().replace("▌  cargo test --workspace", "▌  cargo test --workspace\n▌  Allow `rm`?");

    assert_eq!((ask_screen(&late), ask_screen(&second)), (None, None));
}

#[test]
fn a_line_that_only_looks_like_the_hint_is_part_of_the_command() {
    let other = permission_screen().replace("← → narrow scope  ·  e edit pattern", "← rm -rf target");

    let ask = ask_screen(&other).expect("a permission");

    assert_eq!(ask.detail, "cargo test --workspace\n\n← rm -rf target");
}
