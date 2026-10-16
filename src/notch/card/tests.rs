use std::time::{Duration, Instant};

use rstest::rstest;

use super::*;
use crate::ask::{Ask, Choice};

fn card(kind: Kind) -> Card {
    Card { pane: 7, title: "Fix the auth bug".into(), place: "shop › main".into(), since: None, kind }
}

fn asks(question: bool) -> Kind {
    let choices = if question {
        vec![Choice { label: "Red".into(), key: '1' }, Choice { label: "Green".into(), key: '2' }]
    } else {
        vec![Choice { label: "allow".into(), key: '1' }, Choice { label: "deny".into(), key: '4' }]
    };
    Kind::Asks(Ask {
        question,
        title: if question { "Which colour?" } else { "Bash command" }.into(),
        detail: if question { "Colour" } else { "npm test -- auth" }.into(),
        choices,
    })
}

fn on(pane: u64, kind: Kind) -> Card {
    Card { pane, ..card(kind) }
}

#[rstest]
#[case::done(Kind::Done)]
#[case::waiting(Kind::Waiting)]
#[case::permission(asks(false))]
#[case::question(asks(true))]
fn every_card_opens_its_own_pane(#[case] kind: Kind) {
    let opens: Vec<Press> = buttons(&card(kind)).into_iter().filter(|b| b.label == "open").map(|b| b.press).collect();

    assert_eq!(opens, vec![Press::Open(7)]);
}

#[test]
fn a_permission_offers_allow_in_green_deny_in_pink_then_open() {
    let found: Vec<(String, u32, u32)> =
        buttons(&card(asks(false))).into_iter().map(|b| (b.label, b.fill, b.text)).collect();

    assert_eq!(
        found,
        [
            ("allow".into(), 0x1d_3a_2c, 0x58_e6_a0),
            ("deny".into(), 0x45_20_2b, 0xff_8a_a3),
            ("open".into(), 0x2e_2d_33, 0xff_ff_ff)
        ]
    );
}

#[test]
fn a_choice_answers_with_its_key() {
    let presses: Vec<Press> = buttons(&card(asks(false))).into_iter().map(|b| b.press).collect();

    assert!(matches!(&presses[1], Press::Answer(Action::Answer { pane: 7, key: '4', .. })));
}

#[rstest]
#[case::done(Kind::Done, "Finished", 0x58_e6_a0)]
#[case::waiting(Kind::Waiting, "Needs you", 0xf5_d7_6e)]
#[case::permission(asks(false), "Wants to run: npm test -- auth", 0xf5_d7_6e)]
#[case::question(asks(true), "Which colour?", 0xb2_94_ff)]
fn the_status_line_says_what_the_agent_wants(#[case] kind: Kind, #[case] text: &str, #[case] colour: u32) {
    assert_eq!(status(&card(kind)), (text.to_string(), colour));
}

#[rstest]
#[case::permission(asks(false), Mood::Needs)]
#[case::waiting(Kind::Waiting, Mood::Needs)]
#[case::question(asks(true), Mood::Question)]
#[case::done(Kind::Done, Mood::Done)]
fn rinco_shows_what_each_card_is_about(#[case] kind: Kind, #[case] expected: Mood) {
    assert_eq!(mood(&card(kind)), expected);
}

#[rstest]
#[case::picked(asks(true), '2', Outcome::Picked("Green".into()), "✓ You picked Green", Mood::Done, false)]
#[case::allowed(asks(false), '1', Outcome::Running("npm test -- auth".into()), "◐ Running npm test -- auth…", Mood::Working, true)]
#[case::denied(asks(false), '4', Outcome::Denied, "✕ Denied. Claude will try another way.", Mood::Error, false)]
fn an_answer_shows_what_it_did(
    #[case] kind: Kind,
    #[case] key: char,
    #[case] expected: Outcome,
    #[case] line: &str,
    #[case] then: Mood,
    #[case] monospaced: bool,
) {
    let found = outcome(&card(kind), key).expect("an outcome");

    assert_eq!((found.line().0.as_str(), found.mood(), found.monospaced()), (line, then, monospaced));
    assert_eq!(found, expected);
}

#[rstest]
#[case::question(asks(true), "Fix the auth bug · Which colour?")]
#[case::permission(asks(false), "Fix the auth bug · npm test -- auth")]
#[case::waiting(Kind::Waiting, "Fix the auth bug needs you")]
#[case::done(Kind::Done, "Fix the auth bug finished")]
fn the_pill_says_it_in_one_line(#[case] kind: Kind, #[case] line: &str) {
    assert_eq!(summary(&card(kind)), line);
}

#[test]
fn the_pill_counts_the_others() {
    assert_eq!([1, 3].map(more), [None, Some("+2".into())]);
}

#[test]
fn the_place_shows_only_when_the_title_is_not_it() {
    let mut card = card(Kind::Done);
    let named = subtitle(&card).to_string();
    card.title.clone_from(&card.place);

    assert_eq!((named.as_str(), subtitle(&card)), ("shop › main", ""));
}

#[test]
fn only_a_new_card_that_needs_an_answer_opens_the_panel() {
    let before = [on(1, asks(true))];

    assert!(!arrived(&before, &[on(1, asks(true)), on(2, Kind::Done)]));
    assert!(arrived(&before, &[on(1, asks(true)), on(2, Kind::Waiting)]));
    assert!(arrived(&before, &[on(1, asks(false))]));
}

#[test]
fn done_flashes_and_the_rest_stay_until_answered() {
    assert_eq!([Kind::Done, Kind::Waiting, asks(true)].map(|k| lasts(&card(k))), [Some(DONE_FOR), None, None]);
}

mod expand {
    use super::*;

    #[test]
    fn a_new_question_opens_it_for_a_while() {
        let (mut expand, now) = (Expand::default(), Instant::now());

        expand.arrive(now);

        assert_eq!((expand.at(now + Duration::from_secs(3)), expand.at(now + EXPAND_FOR)), (true, false));
    }

    #[test]
    fn hovering_keeps_it_open_and_leaving_closes_it_later() {
        let (mut expand, now) = (Expand::default(), Instant::now());
        expand.hover(true, now);
        let held = expand.at(now + Duration::from_mins(1));

        expand.hover(false, now + Duration::from_mins(1));

        let left = now + Duration::from_mins(1);
        assert_eq!((held, expand.at(left + EXPAND_FOR / 2), expand.at(left + EXPAND_FOR)), (true, true, false));
    }

    #[test]
    fn it_starts_closed() {
        assert!(!Expand::default().at(Instant::now()));
    }
}

#[rstest]
#[case::just_now(30, "now")]
#[case::minutes(28 * 60, "28m")]
#[case::hours(5 * 3_600 + 100, "5h")]
#[case::days(3 * 86_400, "3d")]
fn ago_is_short(#[case] secs: u64, #[case] text: &str) {
    assert_eq!(ago(secs), text);
}
