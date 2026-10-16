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
    ask.choices.iter().map(|c| (c.label.as_str(), c.key)).collect()
}

#[test]
fn a_permission_prompt_offers_allow_and_deny_with_their_digits() {
    let ask = ask(Some(PERMISSION), &fixture("permission")).expect("a permission prompt");

    assert_eq!(
        (ask.question, ask.title.as_str(), ask.detail.as_str(), choices(&ask)),
        (false, "Bash command", "touch /home/a/web-shop/made-by-claude", vec![("allow", '1'), ("deny", '4')])
    );
}

#[test]
fn a_single_question_offers_its_options_but_not_free_text() {
    let ask = ask(Some(QUESTION), &fixture("question")).expect("a question");

    assert_eq!(
        (ask.question, ask.title.as_str(), ask.detail.as_str(), choices(&ask)),
        (true, "Which colour do you prefer?", "Colour", vec![("Red", '1'), ("Green", '2'), ("Blue", '3')])
    );
}

#[rstest]
#[case::several_questions_and_checkboxes(QUESTION, "questions")]
#[case::the_question_answered(QUESTION, "answered")]
#[case::the_prompt_gone(PERMISSION, "answered")]
#[case::a_permission_named_a_question(QUESTION, "permission")]
#[case::a_question_named_a_permission(PERMISSION, "question")]
fn anything_else_offers_no_answer(#[case] waiting_for: &str, #[case] name: &str) {
    assert_eq!(ask(Some(waiting_for), &fixture(name)), None);
}

#[test]
fn nothing_is_offered_without_knowing_what_claude_waits_for() {
    assert_eq!(ask(None, &fixture("permission")), None);
    assert_eq!(ask(Some("dialog"), &fixture("question")), None);
}

#[test]
fn a_quote_of_the_prompt_in_the_conversation_is_not_the_prompt() {
    let quoted = fixture("permission").lines().map(|line| format!("  {line}")).collect::<Vec<_>>().join("\n");

    assert_eq!(ask(Some(PERMISSION), &quoted), None);
}

#[test]
fn a_prompt_without_a_no_offers_nothing() {
    let screen = fixture("permission").replace("   4. No", "   4. Maybe");

    assert_eq!(ask(Some(PERMISSION), &screen), None);
}

#[test]
fn options_out_of_order_offer_nothing() {
    let screen = fixture("question").replace("  3. Blue", "  7. Blue");

    assert_eq!(ask(Some(QUESTION), &screen), None);
}

#[test]
fn a_wrapped_option_is_read_whole() {
    let screen = fixture("permission").replace(
        "   2. Yes, and always allow access to /home/a/web-shop from this project",
        "   2. Yes, and always allow access to /home/a/web-shop\n      from this project",
    );

    assert!(ask(Some(PERMISSION), &screen).is_some());
}
