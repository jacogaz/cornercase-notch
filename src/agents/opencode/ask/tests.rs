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
fn a_permission_offers_esc_to_reject_and_never_enter_to_allow() {
    let ask = ask_screen(&screen("permission")).expect("a permission");

    assert_eq!(
        (ask.question, ask.title.as_str(), ask.detail.as_str(), choices(&ask)),
        (false, "Shell command", "$ touch /private/tmp/claude-501/ccn/proj/oc.txt", vec![("deny", vec!["esc"])])
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
fn neither_button_row_offers_an_allow() {
    let later = screen("permission").replace("Allow always", "Always allow");

    for screen in [screen("permission"), later] {
        assert_eq!(choices(&ask_screen(&screen).expect("a permission")), [("deny", vec!["esc"])]);
    }
}

#[test]
fn text_after_a_gap_inside_a_prompt_line_gives_no_ask() {
    let bar = "  ┃  $ touch /private/tmp/claude-501/ccn/proj/oc.txt";
    let cut = screen("permission").replace(bar, &format!("{bar}           && rm -rf x"));
    let long_gap = screen("permission").replace(bar, "  ┃  $ touch           /private/tmp/claude-501/ccn/proj/oc.txt");
    let option = screen("question").replace("2. Green", "2. Green          (delete production)");
    let description = screen("question").replace("The colour blue", "The colour blue           (and more)");
    let title = screen("question").replace("Which colour do you prefer?", "Which colour?           Really?");

    for screen in [cut, long_gap, option, description, title] {
        assert_eq!(ask_screen(&screen), None);
    }
}

#[test]
fn indented_content_below_the_prompt_is_not_the_sidebar() {
    for name in ["permission", "question"] {
        let below = format!("{}\n{}another prompt\n", screen(name), " ".repeat(10));

        assert_eq!(ask_screen(&below), None, "{name}");
    }
}

#[test]
fn text_reaching_the_sidebar_may_have_been_cut_by_it() {
    let with = |n: usize| {
        screen("question").replace("  ┃     The colour red", &format!("  ┃     The colour red{}", "x".repeat(n)))
    };

    assert!(ask_screen(&with(100)).is_some());
    assert_eq!(ask_screen(&with(150)), None);
}

#[test]
fn a_screen_without_a_sidebar_is_read_whole() {
    for name in ["permission", "question"] {
        let narrow: String = screen(name).lines().map(|l| l.chars().take(164).collect::<String>() + "\n").collect();

        assert!(ask_screen(&narrow).is_some(), "{name}");
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
fn a_prompt_without_a_sidebar_but_with_text_after_a_gap_gives_no_ask() {
    let narrow: String = screen("question").lines().map(|l| l.chars().take(164).collect::<String>() + "\n").collect();

    assert_eq!(ask_screen(&narrow.replace("2. Green", "2. Green          (delete production)")), None);
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

#[test]
fn the_sidebar_edge_is_the_footer_s_column_and_text_cannot_move_it() {
    let suffixed = screen("question")
        .replace("1. Red", "1. Red                                                  (a)")
        .replace("2. Green", "2. Green                                                  (b)")
        .replace("3. Blue", "3. Blue                                                  (c)");
    let appended = format!("{}\n{}\n{}\n", screen("question"), " ".repeat(60) + "one", " ".repeat(60) + "two");

    assert_eq!((ask_screen(&suffixed), ask_screen(&appended)), (None, None));
}

#[test]
fn a_fake_footer_inside_the_prompt_is_not_the_edge() {
    let fake = screen("question").replace("  ┃  1. Red", "  ┃  1. Red          • OpenCode 9.9.9");

    assert_eq!(ask_screen(&fake), None);
}

#[test]
fn lines_at_column_sixty_before_the_footer_give_no_ask() {
    let real = screen("question");
    let (body, footer) = real.trim_end().rsplit_once('\n').expect("a footer");
    let inserted = format!("{body}\n{}one\n{}two\n{}three\n{footer}\n", " ".repeat(60), " ".repeat(60), " ".repeat(60));

    assert!(ask_screen(&real).is_some());
    assert_eq!(ask_screen(&inserted), None);
}

#[test]
fn the_footer_is_found_only_as_the_last_line_with_a_version() {
    let real = screen("question");
    let column = real.lines().last().and_then(|line| line.chars().position(|c| c == '•'));

    assert_eq!(Some(edge(&real)), column);
    assert_eq!(edge(&real.replace("1.19.3", "2.0.0")), edge(&real));
    assert_eq!(edge(&real.replace("1.19.3", "x")), usize::MAX);
    assert_eq!(edge(&format!("{real}\nlast line")), usize::MAX);
}
