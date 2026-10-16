use rstest::rstest;

use super::*;
use crate::test_util::fixture;

fn fixture(name: &str) -> String {
    match name {
        "permission" => fixture!("claude/2.1.296/prompts/permission.txt"),
        "question" => fixture!("claude/2.1.296/prompts/question.txt"),
        "questions" => fixture!("claude/2.1.296/prompts/questions.txt"),
        _ => fixture!("claude/2.1.296/prompts/answered.txt"),
    }
    .to_string()
}

fn choices(ask: &Ask) -> Vec<(&str, char)> {
    ask.choices.iter().map(|c| (c.label.as_str(), c.keys[0].chars().next().expect("a key"))).collect()
}

#[test]
fn a_permission_prompt_offers_allow_and_deny_with_their_digits() {
    let ask = ask_screen(&fixture("permission")).expect("a permission prompt");

    assert_eq!(
        (ask.question, ask.title.as_str(), ask.detail.as_str(), choices(&ask)),
        (false, "Bash command", "touch /home/a/web-shop/made-by-claude", vec![("allow", '1'), ("deny", '4')])
    );
}

#[test]
fn a_single_question_offers_its_options_but_not_free_text() {
    let ask = ask_screen(&fixture("question")).expect("a question");

    assert_eq!(
        (ask.question, ask.title.as_str(), ask.detail.as_str(), choices(&ask)),
        (true, "Which colour do you prefer?", "Colour", vec![("Red", '1'), ("Green", '2'), ("Blue", '3')])
    );
}

#[rstest]
#[case::several_questions_and_checkboxes("questions")]
#[case::the_question_answered("answered")]
fn anything_else_offers_no_answer(#[case] name: &str) {
    assert_eq!(ask_screen(&fixture(name)), None);
}

#[test]
fn a_quote_of_the_prompt_in_the_conversation_is_not_the_prompt() {
    let quoted = fixture("permission").lines().map(|line| format!("  {line}")).collect::<Vec<_>>().join("\n");

    assert_eq!(ask_screen(&quoted), None);
}

#[test]
fn a_prompt_without_a_no_offers_nothing() {
    let screen = fixture("permission").replace("   4. No", "   4. Maybe");

    assert_eq!(ask_screen(&screen), None);
}

#[test]
fn options_out_of_order_offer_nothing() {
    let screen = fixture("question").replace("  3. Blue", "  7. Blue");

    assert_eq!(ask_screen(&screen), None);
}

#[test]
fn a_wrapped_option_is_read_whole() {
    let screen = fixture("permission").replace(
        "   2. Yes, and always allow access to /home/a/web-shop from this project",
        "   2. Yes, and always allow access to /home/a/web-shop\n      from this project",
    );

    assert!(ask_screen(&screen).is_some());
}

#[test]
fn a_question_from_a_later_claude_code_still_parses() {
    let ask = ask_screen(fixture!("claude/2.1.302/prompts/question.txt")).expect("a question");

    assert_eq!(
        (ask.question, ask.title.as_str(), ask.detail.as_str(), choices(&ask)),
        (true, "Which colour do you prefer?", "Colour", vec![("Red", '1'), ("Green", '2'), ("Blue", '3')])
    );
}

#[rstest]
#[case::a_permission_prompt("permission")]
#[case::a_question("question")]
fn anything_after_the_footer_means_no_ask(#[case] name: &str) {
    for after in ["\nsomething else", "\n\n❯ ", "\n─────────\n Do you want to proceed?"] {
        assert_eq!(ask_screen(&format!("{}{after}", fixture(name).trim_end())), None, "{after:?}");
    }
    assert!(ask_screen(&format!("{}\n\n\n", fixture(name).trim_end())).is_some());
}

#[rstest]
#[case::the_two_prompts_together("permission", "question")]
#[case::the_same_twice("question", "question")]
fn an_obsolete_prompt_above_a_newer_one_is_not_the_ask(#[case] old: &str, #[case] new: &str) {
    let screen = format!("{}\n{}", fixture(old), fixture(new));
    let found = ask_screen(&screen);

    assert_eq!(found, ask_screen(&fixture(new)));
    assert_ne!(found, None);
}

#[test]
fn a_command_with_a_second_line_keeps_its_newline() {
    let screen =
        fixture("permission").replace(" touch /home/a/web-shop/made-by-claude\n", " echo  safe\n rm -rf target\n");

    assert_eq!(ask_screen(&screen).expect("an ask").detail, "echo  safe\nrm -rf target");
}

#[test]
fn a_changed_option_description_changes_the_ask() {
    let changed = fixture("question").replace("The colour green", "Another green");

    assert_ne!(ask_screen(&changed), ask_screen(&fixture("question")));
}

#[test]
fn a_changed_tip_or_header_above_a_permission_changes_the_ask() {
    let changed = fixture("permission").replace("Create empty marker file", "Remove everything");

    assert_ne!(ask_screen(&changed), ask_screen(&fixture("permission")));
}
