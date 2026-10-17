use std::cell::Cell;

use rstest::rstest;

use super::*;
use crate::cornercase::parse_panes;
use crate::test_util::fixture;

const T0: u64 = 1_791_820_000;

fn at(secs: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(T0 + secs)
}

fn panes(rows: &[&str]) -> Vec<Pane> {
    parse_panes(&format!(r#"{{"panes":[{}]}}"#, rows.join(","))).expect("panes")
}

fn claude(pane: u64, tab: u64, status: &str, shown: bool) -> String {
    format!(
        r#"{{"pane":{pane},"tab":{tab},"program":"claude","agent":"claude","status":"{status}","shown":{shown},
        "tab_name":"claude","workspace_name":"main","project_name":"shop"}}"#
    )
}

fn screen(name: &str) -> String {
    match name {
        "permission" => fixture!("claude/2.1.296/prompts/permission.txt"),
        "question" => fixture!("claude/2.1.296/prompts/question.txt"),
        "questions" => fixture!("claude/2.1.296/prompts/questions.txt"),
        _ => fixture!("claude/2.1.296/prompts/answered.txt"),
    }
    .to_string()
}

fn status(pane: u64, from: &str, to: &str) -> Event {
    Event { event: Some("status".into()), pane: Some(pane), from: Some(from.into()), to: Some(to.into()) }
}

fn built(rows: &[&str], tracker: &Tracker, now: SystemTime, shows: &str) -> Built {
    build(&panes(rows), tracker, now, |_| Some(screen(shows)))
}

fn kinds(built: &Built) -> Vec<(u64, &'static str)> {
    built
        .cards
        .iter()
        .map(|e| {
            (
                e.card.pane,
                match &e.card.kind {
                    Kind::Done => "done",
                    Kind::Failed => "failed",
                    Kind::Waiting => "waiting",
                    Kind::Asks(ask) if ask.question => "question",
                    Kind::Asks(_) => "permission",
                },
            )
        })
        .collect()
}

#[rstest]
#[case::permission("permission", "permission")]
#[case::question("question", "question")]
#[case::several_questions("questions", "waiting")]
#[case::the_prompt_gone("answered", "waiting")]
fn a_waiting_claude_asks_when_its_screen_is_a_prompt_cornercase_knows(#[case] shows: &str, #[case] expected: &str) {
    let built = built(&[&claude(4, 1, "waiting", false)], &Tracker::default(), at(0), shows);

    assert_eq!(kinds(&built), [(4, expected)]);
}

#[test]
fn a_permission_card_carries_the_ask_and_the_place() {
    let built = built(&[&claude(4, 1, "waiting", false)], &Tracker::default(), at(0), "permission");

    let card = &built.cards[0].card;
    assert_eq!((card.place.as_str(), card.title.as_str(), card.since), ("shop › main", "shop › main", None));
    let Kind::Asks(ask) = &card.kind else { panic!("asks") };
    assert_eq!((ask.title.as_str(), ask.choices.len()), ("Bash command", 2));
}

#[test]
fn a_screen_that_cannot_be_read_is_just_waiting() {
    let built = build(&panes(&[&claude(4, 1, "waiting", false)]), &Tracker::default(), at(0), |_| None);

    assert_eq!(kinds(&built), [(4, "waiting")]);
}

#[test]
fn screens_are_read_only_for_waiting_panes_of_agents_that_answer_and_working_opencode_and_idle_cursor_panes() {
    let rows = [
        claude(1, 1, "working", false),
        claude(2, 1, "waiting", false),
        claude(3, 1, "done", false),
        claude(4, 1, "idle", false),
        r#"{"pane":5,"tab":2,"agent":"codex","status":"waiting"}"#.to_string(),
        r#"{"pane":7,"tab":2,"agent":"aider","status":"waiting"}"#.to_string(),
        r#"{"pane":8,"tab":2,"agent":"opencode","status":"working"}"#.to_string(),
        r#"{"pane":9,"tab":2,"agent":"cursor","status":"idle"}"#.to_string(),
        r#"{"pane":6,"tab":2,"status":null}"#.to_string(),
    ];
    let read = Cell::new(Vec::new());
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();

    let built = build(&panes(&rows), &Tracker::default(), at(0), |pane| {
        let mut seen = read.take();
        seen.push(pane);
        read.set(seen);
        None
    });

    assert_eq!(
        (read.take(), kinds(&built)),
        (vec![2, 5, 8, 9], vec![(2, "waiting"), (5, "waiting"), (7, "waiting"), (3, "done")])
    );
}

#[rstest]
#[case::working("working")]
#[case::shell("shell")]
#[case::idle("idle")]
#[case::pending("pending")]
fn the_rest_get_no_card(#[case] state: &str) {
    assert_eq!(built(&[&claude(1, 1, state, false)], &Tracker::default(), at(0), "answered").cards, []);
}

#[test]
fn a_card_starts_when_this_process_saw_its_status_begin() {
    let mut tracker = Tracker::default();
    tracker.observe(&status(1, "working", "waiting"), false, false, at(5));
    tracker.observe(&status(2, "working", "done"), false, false, at(9));
    let rows = [claude(1, 1, "waiting", false), claude(2, 1, "done", false), claude(3, 1, "done", false)];
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();

    let built = built(&rows, &tracker, at(20), "answered");

    let since: Vec<_> = built.cards.iter().map(|e| (e.card.pane, e.card.since)).collect();
    assert_eq!(since, [(1, Some(T0 + 5)), (2, Some(T0 + 9)), (3, None)]);
}

#[test]
fn a_status_this_process_missed_is_not_dated() {
    let mut tracker = Tracker::default();
    tracker.observe(&status(1, "working", "waiting"), false, false, at(5));

    let built = built(&[&claude(1, 1, "done", false)], &tracker, at(20), "answered");

    assert_eq!(built.cards[0].card.since, None);
}

#[rstest]
#[case::the_tab_name("Fix the auth bug", "Fix the auth bug")]
#[case::the_program_name("claude", "shop › main")]
#[case::the_program_name_in_any_case("Claude", "shop › main")]
#[case::nothing("", "shop › main")]
fn the_title_is_the_tab_name_unless_it_is_just_the_program(#[case] name: &str, #[case] title: &str) {
    let pane = claude(1, 1, "done", false).replace(r#""tab_name":"claude""#, &format!(r#""tab_name":"{name}""#));

    let built = built(&[&pane], &Tracker::default(), at(0), "answered");

    assert_eq!(built.cards[0].card.title, title);
}

#[test]
fn the_place_uses_what_names_exist() {
    let only_project = claude(1, 1, "done", false).replace(r#""workspace_name":"main""#, r#""workspace_name":null"#);
    let neither = r#"{"pane":2,"tab":1,"agent":"claude","status":"done"}"#;

    let built = built(&[&only_project, neither], &Tracker::default(), at(0), "answered");

    assert_eq!(built.cards.iter().map(|e| e.card.place.as_str()).collect::<Vec<_>>(), ["shop", ""]);
}

#[test]
fn asks_come_first_then_waiting_then_done_newest_first() {
    let mut tracker = Tracker::default();
    for (pane, to, secs) in [(1, "done", 1), (2, "done", 9), (3, "waiting", 3), (4, "waiting", 7)] {
        tracker.observe(&status(pane, "working", to), false, false, at(secs));
    }
    let rows = [
        claude(1, 1, "done", false),
        claude(2, 1, "done", false),
        claude(3, 1, "waiting", false),
        claude(4, 1, "waiting", false),
        claude(5, 1, "waiting", false),
    ];
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let board = build(&panes(&rows), &tracker, at(20), |pane| (pane == 5).then(|| screen("question")));

    assert_eq!(kinds(&board), [(5, "question"), (4, "waiting"), (3, "waiting"), (2, "done"), (1, "done")]);
}

#[test]
fn the_visible_tab_is_the_one_of_the_shown_pane() {
    let rows = [claude(1, 1, "done", false), claude(2, 2, "done", true)];
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();

    assert_eq!(built(&rows, &Tracker::default(), at(0), "answered").visible, Some(2));
    assert_eq!(built(&[&claude(1, 1, "done", false)], &Tracker::default(), at(0), "answered").visible, None);
}

#[test]
fn cards_of_the_visible_tab_are_dropped_only_while_looking() {
    let rows = [claude(1, 1, "done", false), claude(2, 2, "done", true), claude(3, 2, "waiting", false)];
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let built = built(&rows, &Tracker::default(), at(0), "answered");
    let panes = |looking| built.board(looking).cards.iter().map(|c| c.pane).collect::<Vec<_>>();

    assert_eq!((panes(true), panes(false)), (vec![1], vec![3, 1, 2]));
}

const IDLE: &str = "idle";

fn flash(from: &str, to: &str, shown: bool, looking: bool) -> Tracker {
    let mut tracker = Tracker::default();
    tracker.observe(&status(1, from, to), shown, looking, at(0));
    tracker
}

fn flashed(tracker: &Tracker, state: &str, secs: u64) -> Option<(Kind, Option<u64>)> {
    let built = built(&[&claude(1, 1, state, true)], tracker, at(secs), "answered");
    built.cards.first().map(|entry| (entry.card.kind.clone(), entry.card.since))
}

#[rstest]
#[case::working_to_idle("working", "idle")]
#[case::shell_to_idle("shell", "idle")]
#[case::working_to_done("working", "done")]
#[case::shell_to_done("shell", "done")]
fn a_shown_agent_that_finishes_out_of_sight_flashes_done(#[case] from: &str, #[case] to: &str) {
    let tracker = flash(from, to, true, false);

    assert_eq!(flashed(&tracker, to, 3), Some((Kind::Done, Some(T0))));
}

#[test]
fn the_flash_ends_after_ten_seconds() {
    let tracker = flash("working", "idle", true, false);

    assert_eq!(
        (
            flashed(&tracker, IDLE, 9).is_some(),
            flashed(&tracker, IDLE, 10),
            tracker.flashing(at(9)),
            tracker.flashing(at(10))
        ),
        (true, None, true, false)
    );
}

#[test]
fn working_or_waiting_again_ends_the_flash() {
    for next in ["working", "shell", "waiting"] {
        let mut tracker = flash("working", "idle", true, false);
        tracker.observe(&status(1, "idle", next), true, false, at(2));

        assert_eq!(flashed(&tracker, IDLE, 3), None, "{next}");
    }
}

#[test]
fn a_flash_needs_a_shown_pane_and_a_window_nobody_looks_at() {
    assert_eq!(flashed(&flash("working", "idle", false, false), IDLE, 1), None);
    assert_eq!(flashed(&flash("working", "idle", true, true), IDLE, 1), None);
}

#[test]
fn other_changes_do_not_flash() {
    assert_eq!(flashed(&flash("waiting", "idle", true, false), IDLE, 1), None);
    assert_eq!(flashed(&flash("idle", "done", true, false), "idle", 1), None);
}

#[test]
fn a_closed_pane_is_forgotten() {
    let mut tracker = flash("working", "idle", true, false);

    tracker.retain(&panes(&[&claude(2, 1, "idle", true)]));

    assert!(!tracker.flashing(at(1)));
}

fn ask_of(name: &str) -> Ask {
    ask_screen("claude", &screen(name)).expect("an ask")
}

fn waiting() -> Pane {
    panes(&[&claude(4, 1, "waiting", true)]).remove(0)
}

#[test]
fn an_answer_is_typed_only_while_the_same_ask_is_on_the_screen() {
    let ask = ask_of("permission");

    assert!(still_asks(&ask, 0, Some(&waiting()), || Some(screen("permission"))));
    assert!(still_asks(&ask, 1, Some(&waiting()), || Some(screen("permission"))));
}

#[test]
fn an_answer_is_refused_when_anything_changed() {
    let ask = ask_of("permission");
    let working = panes(&[&claude(4, 1, "working", true)]).remove(0);
    let codex = panes(&[r#"{"pane":4,"tab":1,"agent":"codex","status":"waiting"}"#]).remove(0);

    let refused = [
        still_asks(&ask, 0, None, || Some(screen("permission"))),
        still_asks(&ask, 0, Some(&working), || Some(screen("permission"))),
        still_asks(&ask, 0, Some(&codex), || Some(screen("permission"))),
        still_asks(&ask, 0, Some(&waiting()), || Some(screen("answered"))),
        still_asks(&ask, 0, Some(&waiting()), || Some(screen("question"))),
        still_asks(&ask, 0, Some(&waiting()), || None),
        still_asks(&ask, 5, Some(&waiting()), || Some(screen("permission"))),
        still_asks(&ask, 9, Some(&waiting()), || Some(screen("permission"))),
    ];

    assert_eq!(refused, [false; 8]);
}

#[test]
fn the_screen_is_not_read_when_the_pane_already_disagrees() {
    let ask = ask_of("permission");
    let working = panes(&[&claude(4, 1, "working", true)]).remove(0);

    assert!(!still_asks(&ask, 0, Some(&working), || panic!("read a screen")));
}

fn agent(pane: u64, name: &str, status: &str) -> String {
    format!(
        r#"{{"pane":{pane},"tab":1,"program":"{name}","agent":"{name}","status":"{status}","project_name":"shop","workspace_name":"main"}}"#
    )
}

const AGENTS: [&str; 7] = ["claude", "codex", "gemini", "opencode", "grok", "cursor", "aider"];

#[test]
fn every_agent_gets_waiting_failed_and_done_cards_and_none_for_the_rest() {
    let mut rows = Vec::new();
    for (i, name) in AGENTS.iter().enumerate() {
        let base = 10 * (u64::try_from(i).expect("small") + 1);
        for (n, status) in ["working", "shell", "idle", "pending", "waiting", "failed", "done"].iter().enumerate() {
            rows.push(agent(base + u64::try_from(n).expect("small"), name, status));
        }
    }
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();

    let built = build(&panes(&rows), &Tracker::default(), at(0), |_| None);

    let by_agent: Vec<(String, Vec<&str>)> = AGENTS
        .iter()
        .map(|name| {
            let shown = built.cards.iter().filter(|e| e.card.agent == label(name)).map(|e| match e.card.kind {
                Kind::Waiting => "waiting",
                Kind::Failed => "failed",
                Kind::Done => "done",
                Kind::Asks(_) => "asks",
            });
            (label(name), shown.collect())
        })
        .collect();
    assert!(by_agent.iter().all(|(_, shown)| shown == &["waiting", "failed", "done"]), "{by_agent:?}");
}

#[test]
fn only_agents_that_answer_get_buttons_for_a_waiting_screen() {
    let rows = [agent(1, "claude", "waiting"), agent(2, "codex", "waiting")];
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();

    let built = build(&panes(&rows), &Tracker::default(), at(0), |_| Some(screen("permission")));

    assert_eq!(kinds(&built), [(1, "permission"), (2, "waiting")]);
}

#[test]
fn a_pane_without_an_agent_gets_no_card() {
    let rows = [r#"{"pane":1,"tab":1,"agent":"","status":"done"}"#, r#"{"pane":2,"tab":1,"status":"done"}"#];

    assert_eq!(build(&panes(&rows), &Tracker::default(), at(0), |_| None).cards, []);
}

#[test]
fn a_card_names_its_agent() {
    let rows = [agent(1, "gemini", "done"), agent(2, "aider", "done")];
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();

    let built = build(&panes(&rows), &Tracker::default(), at(0), |_| None);

    assert_eq!(built.cards.iter().map(|e| e.card.agent.as_str()).collect::<Vec<_>>(), ["Gemini CLI", "aider"]);
}

#[test]
fn failed_comes_after_waiting_and_before_done() {
    let mut tracker = Tracker::default();
    for (pane, to, secs) in [(1, "done", 9), (2, "failed", 1), (3, "failed", 5), (4, "waiting", 2)] {
        tracker.observe(&status(pane, "working", to), false, false, at(secs));
    }
    let rows = [
        agent(1, "codex", "done"),
        agent(2, "gemini", "failed"),
        agent(3, "grok", "failed"),
        agent(4, "cursor", "waiting"),
    ];
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();

    let built = build(&panes(&rows), &tracker, at(20), |_| None);

    assert_eq!(kinds(&built), [(4, "waiting"), (3, "failed"), (2, "failed"), (1, "done")]);
}

#[test]
fn every_agent_flashes_done_when_it_finishes_out_of_sight() {
    for name in AGENTS {
        let mut tracker = Tracker::default();
        tracker.observe(&status(1, "working", "idle"), true, false, at(0));
        let rows = [agent(1, name, "idle")];
        let rows: Vec<&str> = rows.iter().map(String::as_str).collect();

        let built = build(&panes(&rows), &tracker, at(3), |_| None);

        assert_eq!(kinds(&built), [(1, "done")], "{name}");
    }
}

#[test]
fn an_answer_is_refused_when_anything_the_user_saw_changed() {
    let codex = || panes(&[r#"{"pane":4,"tab":1,"agent":"codex","status":"waiting"}"#]).remove(0);
    let shown = include_str!("../../tests/fixtures/codex/0.163.0/prompts/permission.txt");
    let ask = ask_screen("codex", shown).expect("an ask");

    let same = still_asks(&ask, 0, Some(&codex()), || Some(shown.to_string()));
    let other_environment = still_asks(&ask, 0, Some(&codex()), || Some(shown.replace("local", "remote")));

    assert_eq!((same, other_environment), (true, false));
}

#[test]
fn a_working_opencode_with_a_prompt_on_screen_gets_an_ask_card_and_without_one_none() {
    let rows = [agent(1, "opencode", "working"), agent(2, "opencode", "working"), agent(3, "claude", "working")];
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let prompt = include_str!("../../tests/fixtures/opencode/1.19.3/prompts/permission.txt");

    let built = build(&panes(&rows), &Tracker::default(), at(0), |pane| (pane == 1).then(|| prompt.to_string()));

    assert_eq!(kinds(&built), [(1, "permission")]);
    assert_eq!(built.cards[0].card.since, None);
}

#[test]
fn a_working_opencode_prompt_can_be_answered_but_a_working_claude_cannot() {
    let prompt = include_str!("../../tests/fixtures/opencode/1.19.3/prompts/permission.txt");
    let ask = ask_screen("opencode", prompt).expect("an ask");
    let opencode = panes(&[&agent(1, "opencode", "working")]).remove(0);
    let claude = panes(&[&agent(1, "claude", "working")]).remove(0);

    assert_eq!(
        [&opencode, &claude].map(|pane| still_asks(&ask, 0, Some(pane), || Some(prompt.to_string()))),
        [true, false]
    );
}

#[test]
fn an_idle_cursor_pane_gets_a_card_only_while_its_screen_shows_a_prompt() {
    let rows = panes(&[r#"{"pane":9,"tab":2,"agent":"cursor","status":"idle"}"#]);
    let prompt = include_str!("../../tests/fixtures/cursor/2026.01.23/prompts/permission.txt");
    let chat = include_str!("../../tests/fixtures/cursor/2026.01.23/prompts/chat-question.txt");

    let asking = build(&rows, &Tracker::default(), at(0), |_| Some(prompt.to_string()));
    let quiet = build(&rows, &Tracker::default(), at(0), |_| Some(chat.to_string()));

    assert_eq!((kinds(&asking), kinds(&quiet)), (vec![(9, "permission")], vec![]));
}

#[test]
fn a_prompt_on_a_polled_pane_cancels_the_done_flash_its_own_working_to_idle_change_started() {
    let rows = panes(&[r#"{"pane":9,"tab":2,"agent":"cursor","status":"idle","shown":true}"#]);
    let prompt = include_str!("../../tests/fixtures/cursor/2026.01.23/prompts/permission.txt");
    let mut tracker = Tracker::default();
    tracker.observe(&status(9, "working", "idle"), true, false, at(0));

    let asking = build(&rows, &tracker, at(1), |_| Some(prompt.to_string()));
    tracker.settle(&asking);
    let denied = build(&rows, &tracker, at(2), |_| Some(String::new()));

    assert_eq!((kinds(&asking), kinds(&denied), tracker.flashing(at(2))), (vec![(9, "permission")], vec![], false));
}
