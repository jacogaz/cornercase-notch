use super::*;

#[test]
fn panes_are_read_by_name_and_unknown_fields_are_ignored() {
    let json = r#"{"panes":[{"pane":4,"tab":3,"workspace":2,"project":1,"program":"claude","agent":"claude",
        "status":"waiting","background_shell":false,"model":"opus","context":12,"shown":true,
        "tab_name":"Fix auth","workspace_name":"main","project_name":"shop","brand_new":[1,2]}]}"#;

    let panes = parse_panes(json).expect("panes");

    assert_eq!(
        panes,
        [Pane {
            pane: 4,
            tab: 3,
            agent: Some("claude".into()),
            status: Some("waiting".into()),
            program: Some("claude".into()),
            tab_name: Some("Fix auth".into()),
            workspace_name: Some("main".into()),
            project_name: Some("shop".into()),
            shown: Some(true),
        }]
    );
}

#[test]
fn nulls_missing_fields_and_wrong_types_are_tolerated() {
    let json = r#"{"panes":[
        {"pane":1,"tab":1,"agent":null,"status":7,"shown":"yes"},
        {"pane":2,"tab":1},
        {"pane":"x","tab":1},
        {"tab":1}
    ]}"#;

    let panes = parse_panes(json).expect("panes");

    assert_eq!(
        panes.iter().map(|p| (p.pane, p.agent.as_deref(), p.status.as_deref(), p.shown)).collect::<Vec<_>>(),
        [(1, None, None, None), (2, None, None, None)]
    );
}

#[test]
fn no_panes_is_an_empty_list_and_garbage_is_an_error() {
    assert_eq!(parse_panes("{}").expect("empty"), []);
    assert!(parse_panes("not json").is_err());
}

#[test]
fn a_status_event_gives_its_pane_and_states() {
    let line = r#"{"time":"2026-10-16T09:30:00.123Z","event":"status","agent":"claude","from":"working","to":"waiting","project":1,"workspace":2,"tab":3,"pane":4}"#;

    assert_eq!(
        parse_event(line),
        Some(Event {
            event: Some("status".into()),
            pane: Some(4),
            from: Some("working".into()),
            to: Some("waiting".into())
        })
    );
}

#[test]
fn other_events_and_odd_lines_do_not_break_parsing() {
    assert_eq!(
        parse_event(r#"{"event":"dropped","count":12}"#),
        Some(Event { event: Some("dropped".into()), ..Event::default() })
    );
    assert_eq!(parse_event("pane 5 claude working → waiting"), None);
}
