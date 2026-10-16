use rstest::rstest;

use super::*;
use crate::test_util::fixture;

fn screen() -> String {
    fixture!("codex/0.163.0/prompts/permission.txt").to_string()
}

fn choices(ask: &Ask) -> Vec<(&str, char)> {
    ask.choices.iter().map(|c| (c.label.as_str(), c.key)).collect()
}

#[test]
fn a_command_approval_offers_allow_and_deny_with_their_digits() {
    let ask = ask_screen(&screen()).expect("a command approval");

    assert_eq!(
        (ask.question, ask.title.as_str(), ask.detail.as_str(), choices(&ask)),
        (false, "Run command", "touch ~/.cornercase-notch-probe", vec![("allow", '1'), ("deny", '3')])
    );
}

#[test]
fn the_dont_ask_again_row_is_never_the_allow() {
    let screen = screen().replace("› 1. Yes, proceed (y)", "  1. Yes, and don't ask again (a)");

    assert_eq!(ask_screen(&screen), None);
}

#[test]
fn a_network_approval_allows_just_this_once() {
    let screen = screen().replace("Yes, proceed (y)", "Yes, just this once (y)");

    assert_eq!(choices(&ask_screen(&screen).expect("an approval")), [("allow", '1'), ("deny", '3')]);
}

#[test]
fn the_older_hint_and_the_thread_footer_are_accepted() {
    let screen = screen()
        .replace("differently (esc)", "differently (n)")
        .replace("esc to cancel", "esc to cancel or o to open thread");

    assert!(ask_screen(&screen).is_some());
}

#[test]
fn a_cursor_on_another_row_is_fine() {
    let screen = screen().replace("› 1. Yes, proceed", "  1. Yes, proceed").replace("  3. No, and", "› 3. No, and");

    assert!(ask_screen(&screen).is_some());
}

#[test]
fn a_command_keeps_the_screens_line_boundaries() {
    let screen = screen().replace("  $ touch ~/.cornercase-notch-probe", "  $ touch ~/.cornercase-notch\n    -probe");

    assert_eq!(ask_screen(&screen).expect("an approval").detail, "touch ~/.cornercase-notch\n-probe");
}

#[test]
fn an_elided_command_offers_deny_only() {
    let screen = screen().replace("\n  Reason:", "\n  [… 34 lines] ⌃t view all\n\n  Reason:");

    let ask = ask_screen(&screen).expect("an approval");

    assert_eq!(choices(&ask), [("deny", '3')]);
}

#[test]
fn a_command_cut_off_entirely_still_offers_deny_only() {
    let screen = screen().replace("  $ touch ~/.cornercase-notch-probe", "  [… 34 lines] ⌃t view all");

    let ask = ask_screen(&screen).expect("an approval");

    assert_eq!((ask.detail.as_str(), choices(&ask)), ("", vec![("deny", '3')]));
}

#[rstest]
#[case::no_deny_row("  3. No, and tell Codex what to do differently (esc)", "  3. Maybe later (esc)")]
#[case::two_cursors("  3. No, and", "› 3. No, and")]
#[case::a_number_skipped("  3. No, and", "  4. No, and")]
#[case::a_different_title(
    "Would you like to run the following command?",
    "Would you like to make the following edits?"
)]
#[case::a_different_footer("Press enter to confirm or esc to cancel", "Press enter to confirm or esc to cancel or x")]
#[case::unknown_text_above_the_command("  Environment: local", "  Surprise: local")]
fn anything_else_offers_no_answer(#[case] from: &str, #[case] to: &str) {
    assert_eq!(ask_screen(&screen().replace(from, to)), None);
}

#[test]
fn a_prompt_that_is_not_the_last_thing_on_screen_is_not_a_prompt() {
    let screen = format!("{}\n  something Codex printed afterwards\n", screen());

    assert_eq!(ask_screen(&screen), None);
}

#[test]
fn a_quote_of_the_prompt_in_the_conversation_is_not_the_prompt() {
    let quoted = screen().lines().map(|line| format!("  {line}")).collect::<Vec<_>>().join("\n");

    assert_eq!(ask_screen(&quoted), None);
}

#[test]
fn another_kind_of_screen_is_not_a_command_approval() {
    let question = "  Question 1/1 (1 unanswered)\n  Choose an option.\n\n  › 1. Option 1           First choice.\n    2. None of the above  Optionally, add details in notes (tab)\n\n  tab to add notes | enter to submit answer | esc to interrupt\n";

    assert_eq!(ask_screen(question), None);
    assert_eq!(ask_screen(""), None);
}

#[test]
fn a_command_with_a_second_line_keeps_its_newline_and_spacing() {
    let screen = screen().replace("  $ touch ~/.cornercase-notch-probe", "  $ echo  safe\n    rm -rf target");

    assert_eq!(ask_screen(&screen).expect("an approval").detail, "echo  safe\nrm -rf target");
}

#[rstest]
#[case::environment("Environment: local", "Environment: remote")]
#[case::reason("Reason: Do you approve", "Reason: Do you really approve")]
fn what_the_user_saw_above_the_command_is_part_of_the_ask(#[case] from: &str, #[case] to: &str) {
    let before = ask_screen(&screen()).expect("an approval");
    let after = ask_screen(&screen().replace(from, to)).expect("an approval");

    assert_ne!(before, after);
    assert_eq!((&before.detail, &before.choices), (&after.detail, &after.choices));
}

#[test]
fn a_header_the_card_does_not_show_is_in_the_fingerprint() {
    let with_thread = screen().replace("  Environment: local", "  Environment: local\n\n  Thread: other");

    assert_ne!(ask_screen(&with_thread), ask_screen(&screen()));
}
