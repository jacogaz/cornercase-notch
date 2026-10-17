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

/// `replace` on the lines holding `from`, keeping each line as wide as it was so the box stays a box.
fn swap(screen: &str, from: &str, to: &str) -> String {
    let lines: Vec<String> = screen
        .lines()
        .map(|line| match line.strip_suffix('│').filter(|_| line.contains(from)) {
            Some(text) => {
                let width = text.chars().count();
                let text = text.trim_end().replace(from, to);
                format!("{text}{}│", " ".repeat(width.saturating_sub(text.chars().count())))
            }
            None => line.to_string(),
        })
        .collect();
    lines.join("\n")
}

fn reading(name: &str) -> Option<Ask> {
    ask_screen(match name {
        "permission.txt" => fixture!("cursor/2026.01.23/prompts/permission.txt"),
        "permission-2.txt" => fixture!("cursor/2026.01.23/prompts/permission-2.txt"),
        "reject-reason.txt" => fixture!("cursor/2026.01.23/prompts/reject-reason.txt"),
        _ => fixture!("cursor/2026.01.23/prompts/chat-question.txt"),
    })
}

#[rstest]
#[case::first("permission.txt", "touch /private/tmp/claude-501/ccn/proj/cu.txt")]
#[case::second("permission-2.txt", "touch /private/tmp/claude-501/ccn/proj/cu2.txt")]
fn a_live_command_prompt_offers_y_to_run_once_and_q_to_reject(#[case] name: &str, #[case] command: &str) {
    let ask = reading(name).expect("a prompt");

    assert_eq!(
        (ask.question, ask.title.as_str(), ask.detail.as_str(), choices(&ask)),
        (false, "Run this command?", command, vec![("allow", "y".to_string()), ("deny", "q".to_string())])
    );
}

#[test]
fn the_allowlist_and_run_everything_rows_and_n_are_never_offered() {
    let ask = reading("permission.txt").expect("a prompt");

    assert!(ask.choices.iter().all(|c| c.keys == ["y"] || c.keys == ["q"]));
}

#[test]
fn the_reject_reason_box_and_a_question_written_as_chat_give_no_ask() {
    assert_eq!((reading("reject-reason.txt"), reading("chat-question.txt")), (None, None));
}

#[test]
fn the_fingerprint_holds_both_boxes() {
    let ask = reading("permission.txt").expect("a prompt");

    assert!(ask.fingerprint.contains("$  touch") && ask.fingerprint.contains("Skip (esc or n)"));
    assert_ne!(Some(ask), reading("permission-2.txt"));
}

#[test]
fn without_the_command_box_the_not_in_allowlist_line_is_the_command() {
    let screen = permission_screen();
    let lines: Vec<&str> = screen.lines().collect();
    let cut = lines.iter().position(|l| l.contains("$  touch")).expect("the command box");
    let screen = [&lines[..cut - 1], &lines[cut + 2..]].concat().join("\n");

    let ask = ask_screen(&screen).expect("a prompt");

    assert_eq!((ask.detail.as_str(), choices(&ask).len()), ("touch /private/tmp/claude-501/ccn/proj/cu.txt", 2));
}

#[test]
fn a_command_that_reaches_the_right_border_could_be_cut_so_it_is_not_offered_for_allow() {
    let screen: Vec<String> = permission_screen()
        .lines()
        .map(|line| match (line.contains("$  touch"), line.trim_end().strip_suffix('│')) {
            (true, Some(text)) => format!("{}{}│", text.trim_end(), "x".repeat(text.len() - text.trim_end().len())),
            _ => line.to_string(),
        })
        .collect();

    assert_eq!(choices(&ask_screen(&screen.join("\n")).expect("a prompt")), [("deny", "q".to_string())]);
}

#[rstest]
#[case::text_after_the_box("\n  Auto · 8.5%\n")]
#[case::another_box_after("\n┌──┐\n│ → Add a follow-up │\n└──┘\n")]
fn anything_after_the_decision_box_gives_no_ask(#[case] after: &str) {
    assert_eq!(ask_screen(&format!("{}{after}", permission_screen())), None);
}

#[rstest]
#[case::no_skip_row("Skip (esc or n)", "Skip (esc)")]
#[case::another_skip_row("Skip (esc or n)", "Skip (n)")]
#[case::another_title("Run this command?", "Run this thing?")]
fn a_prompt_that_is_not_the_known_one_gives_no_ask(#[case] from: &str, #[case] to: &str) {
    assert_eq!(ask_screen(&swap(&permission_screen(), from, to)), None);
}

#[test]
fn without_a_run_row_only_reject_is_offered() {
    let ask = ask_screen(&swap(&permission_screen(), "Run (once) (y)", "Run (once)")).expect("a prompt");

    assert_eq!(choices(&ask), [("deny", "q".to_string())]);
}

#[rstest]
#[case::delete("Run this command?", "Delete this file?", "Run (once) (y)", "Delete (y)")]
#[case::web_search("Run this command?", "Allow this web search?", "Run (once) (y)", "Allow search (y)")]
fn the_known_titles_and_rows_are_read(
    #[case] title: &str,
    #[case] other_title: &str,
    #[case] allow: &str,
    #[case] other_allow: &str,
) {
    let ask = ask_screen(&swap(&swap(&permission_screen(), title, other_title), allow, other_allow)).expect("a prompt");

    assert_eq!(
        (ask.title.as_str(), choices(&ask)),
        (other_title, vec![("allow", "y".to_string()), ("deny", "q".to_string())])
    );
}

fn long_screen() -> String {
    fixture!("cursor/2026.01.23/prompts/permission-long.txt").to_string()
}

fn long_command() -> String {
    format!("touch /private/tmp/claude-501/ccn/proj/long-{}-{}.txt", "x".repeat(120), "x".repeat(120))
}

#[test]
fn a_long_command_wrapped_in_both_places_is_offered_for_allow_reassembled_exactly() {
    let ask = ask_screen(&long_screen()).expect("a prompt");

    assert_eq!(
        (ask.detail.clone(), choices(&ask)),
        (long_command(), vec![("allow", "y".to_string()), ("deny", "q".to_string())])
    );
}

#[test]
fn the_wrapped_not_in_allowlist_line_alone_is_reassembled_the_same() {
    let screen = long_screen();
    let lines: Vec<&str> = screen.lines().collect();
    let cut = lines.iter().position(|l| l.contains("$  touch /private/tmp/claude-501/ccn/proj/long")).expect("box");
    let screen = [&lines[..cut - 1], &lines[cut + 2..]].concat().join("\n");

    let ask = ask_screen(&screen).expect("a prompt");

    assert_eq!((ask.detail, ask.choices.len()), (long_command(), 2));
}

#[test]
fn a_command_box_and_a_not_in_allowlist_line_that_disagree_give_no_allow() {
    let screen = long_screen();
    let at = screen.rfind("xxx.txt").expect("the last line");
    let screen = format!("{}xxy.txt{}", &screen[..at], &screen[at + 7..]);

    assert_eq!(choices(&ask_screen(&screen).expect("a prompt")), [("deny", "q".to_string())]);
}

#[test]
fn a_wrapped_line_that_stops_short_of_the_border_could_be_a_newline_so_it_gives_no_allow() {
    let screen = long_screen().replacen("x │\n │    ", "  │\n │    ", 1);

    assert_eq!(choices(&ask_screen(&screen).expect("a prompt")), [("deny", "q".to_string())]);
}

#[rstest]
#[case::unicode("…")]
#[case::dots("...")]
fn a_command_line_ending_in_an_ellipsis_gives_no_allow(#[case] mark: &str) {
    let screen = swap(&permission_screen(), "cu.txt", &format!("cu{mark}"));

    assert_eq!(choices(&ask_screen(&screen).expect("a prompt")), [("deny", "q".to_string())]);
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
