use super::*;
use crate::ask::Choice;

#[test]
fn cards_and_actions_cross_the_pipe_as_json_lines() {
    let ask = Ask {
        question: true,
        title: "Which colour?".into(),
        detail: "Colour".into(),
        choices: vec![Choice { label: "Red".into(), key: '1' }],
    };
    let action = Action::Answer { pane: 4, ask: ask.clone(), key: '1' };
    let card = Card {
        pane: 4,
        title: "Colour preference".into(),
        agent: "Claude Code".into(),
        place: "shop › main".into(),
        since: Some(1_791_820_339),
        kind: Kind::Asks(ask),
    };
    let board = Board { cards: vec![card] };

    let line = serde_json::to_string(&(board.clone(), action.clone())).expect("encode");

    assert!(!line.contains('\n'));
    assert_eq!(serde_json::from_str::<(Board, Action)>(&line).expect("decode"), (board, action));
}

#[test]
fn questions_come_first_then_waiting_then_failed_then_done() {
    let ask = Ask { question: true, title: "Which colour?".into(), detail: String::new(), choices: Vec::new() };
    let mut kinds = vec![Kind::Done, Kind::Failed, Kind::Asks(ask.clone()), Kind::Waiting];
    kinds.sort_by_key(Kind::rank);

    assert_eq!(kinds, [Kind::Asks(ask), Kind::Waiting, Kind::Failed, Kind::Done]);
}
