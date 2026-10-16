use rstest::rstest;

use super::*;
use crate::test_util::fixture;

fn screen(name: &str) -> String {
    match name {
        "permission" => fixture!("opencode/1.19.3/prompts/permission.txt"),
        "confirm" => fixture!("opencode/1.19.3/prompts/always-confirm.txt"),
        _ => fixture!("opencode/1.19.3/prompts/question.txt"),
    }
    .to_string()
}

fn choices(ask: &Ask) -> Vec<(&str, Vec<&str>)> {
    ask.choices.iter().map(|c| (c.label.as_str(), c.keys.iter().map(String::as_str).collect())).collect()
}

#[test]
fn a_permission_offers_enter_to_allow_once_and_esc_to_reject() {
    let ask = ask_screen(&screen("permission")).expect("a permission");

    assert_eq!(
        (ask.question, ask.title.as_str(), ask.detail.as_str(), choices(&ask)),
        (
            false,
            "Shell command",
            "$ touch /private/tmp/claude-501/ccn/proj/oc.txt",
            vec![("allow", vec!["enter"]), ("deny", vec!["esc"])]
        )
    );
}

#[test]
fn the_sidebar_on_the_same_lines_is_not_part_of_the_ask() {
    let ask = ask_screen(&screen("permission")).expect("a permission");

    assert!(!ask.fingerprint.contains("Connect provider") && !ask.fingerprint.contains("OpenCode"));
    let other_sidebar = screen("permission").replace("Connect provider", "Another sidebar!");
    assert_eq!(ask_screen(&other_sidebar), Some(ask));
}

#[test]
fn a_question_offers_one_digit_per_option_but_not_the_free_text_row() {
    let ask = ask_screen(&screen("question")).expect("a question");

    assert_eq!(
        (ask.question, ask.title.as_str(), choices(&ask)),
        (true, "Which colour do you prefer?", vec![("Red", vec!["1"]), ("Green", vec!["2"]), ("Blue", vec!["3"])])
    );
}

#[test]
fn a_changed_option_description_changes_the_ask() {
    let changed = screen("question").replace("The colour green", "Another green");

    assert_ne!(ask_screen(&changed), ask_screen(&screen("question")));
}

#[test]
fn only_a_screen_with_the_always_allow_confirmation_stage_may_allow() {
    let later = screen("permission").replace("Allow always", "Always allow");

    let ask = ask_screen(&later).expect("a permission");

    assert_eq!(choices(&ask), [("deny", vec!["esc"])]);
}

#[test]
fn a_command_that_could_have_been_cut_by_the_sidebar_cannot_be_allowed() {
    let bar = "  ┃  $ touch /private/tmp/claude-501/ccn/proj/oc.txt";
    let cut = screen("permission").replace(bar, &format!("{bar}           && rm -rf x"));
    let long_gap = screen("permission").replace(bar, "  ┃  $ touch           /private/tmp/claude-501/ccn/proj/oc.txt");

    for screen in [cut, long_gap] {
        let ask = ask_screen(&screen).expect("a permission");
        assert_eq!(choices(&ask), [("deny", vec!["esc"])]);
    }
}

#[rstest]
#[case::nothing_after_the_prompt_may_have_text("permission", "  ┃  something else")]
#[case::a_stray_line_below("permission", "an error line")]
fn anything_after_the_prompt_means_no_ask(#[case] name: &str, #[case] extra: &str) {
    assert_eq!(ask_screen(&format!("{}\n{extra}\n", screen(name))), None);
}

#[rstest]
#[case::the_confirmation_stage("confirm")]
fn other_screens_are_not_a_prompt(#[case] name: &str) {
    assert_eq!(ask_screen(&screen(name)), None);
}

#[test]
fn a_changed_button_row_or_title_gives_no_ask() {
    for (from, to) in [
        ("Allow once   Allow always   Reject", "Allow once   Reject"),
        ("△ Permission required", "△ Something else"),
        ("enter confirm", "enter choose"),
    ] {
        assert_eq!(ask_screen(&screen("permission").replace(from, to)), None, "{from}");
    }
}

#[test]
fn a_multiple_choice_question_or_one_without_its_footer_gives_no_ask() {
    let multiple = screen("question")
        .replace("Which colour do you prefer?", "Which colours do you prefer? (select all that apply)");
    let footerless = screen("question").replace("↑↓ select  enter submit  esc dismiss", "↑↓ select");

    assert_eq!((ask_screen(&multiple), ask_screen(&footerless)), (None, None));
}

#[test]
fn a_question_below_another_prompt_or_a_tab_strip_gives_no_ask() {
    let strip = screen("question")
        .replace("  ┃  Which colour do you prefer?", "  ┃  Colour  Size\n  ┃  Which colour do you prefer?");

    assert_eq!(ask_screen(&strip), None);
}

#[test]
fn a_question_without_a_free_text_row_offers_every_row() {
    let without = screen("question").replace("  ┃  4. Type your own answer", "  ┃");

    let ask = ask_screen(&without).expect("a question");

    assert_eq!(ask.choices.len(), 3);
}

#[test]
fn the_stage_a_press_may_lead_to_is_told_apart_for_backing_out() {
    assert_eq!([screen("confirm"), screen("permission"), screen("question")].map(|s| stage(&s)), [true, false, false]);
    let reject = screen("confirm").replace("△ Always allow", "△ Reject permission");
    assert!(stage(&reject));
}
