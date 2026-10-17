use std::cmp::Reverse;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::agents::{ask_screen, label, polled, reads_screen};
use crate::ask::Ask;
use crate::cornercase::{Event, Pane};
use crate::notch::{Board, Card, Kind};

const FLASH_FOR: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Entry {
    pub(crate) tab: u64,
    pub(crate) card: Card,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub(crate) struct Built {
    pub(crate) cards: Vec<Entry>,
    pub(crate) visible: Option<u64>,
}

impl Built {
    pub(crate) fn board(&self, looking: bool) -> Board {
        let hidden = |tab: u64| !(looking && self.visible == Some(tab));
        Board { cards: self.cards.iter().filter(|entry| hidden(entry.tab)).map(|entry| entry.card.clone()).collect() }
    }

    pub(crate) fn waiting(&self) -> bool {
        self.cards.iter().any(|entry| entry.card.kind != Kind::Done)
    }
}

#[derive(Debug, Default)]
pub(crate) struct Tracker {
    since: HashMap<u64, (String, SystemTime)>,
    flashes: HashMap<u64, SystemTime>,
}

impl Tracker {
    pub(crate) fn observe(&mut self, event: &Event, shown: bool, looking: bool, now: SystemTime) {
        let (Some("status"), Some(pane), Some(to)) = (event.event.as_deref(), event.pane, event.to.as_deref()) else {
            return;
        };
        self.since.insert(pane, (to.to_string(), now));
        let over = matches!(to, "done" | "idle");
        if over && matches!(event.from.as_deref(), Some("working" | "shell")) && shown && !looking {
            self.flashes.insert(pane, now);
        } else if !over {
            self.flashes.remove(&pane);
        }
    }

    pub(crate) fn retain(&mut self, panes: &[Pane]) {
        let open = |pane: &u64| panes.iter().any(|p| p.pane == *pane);
        self.since.retain(|pane, _| open(pane));
        self.flashes.retain(|pane, _| open(pane));
    }

    pub(crate) fn flashing(&self, now: SystemTime) -> bool {
        self.flashes.values().any(|at| flash_live(*at, now))
    }

    fn started(&self, pane: &Pane, status: &str) -> Option<u64> {
        let (seen, at) = self.since.get(&pane.pane)?;
        (seen == status).then(|| secs(*at)).flatten()
    }
}

fn flash_live(at: SystemTime, now: SystemTime) -> bool {
    now.duration_since(at).is_ok_and(|age| age < FLASH_FOR)
}

fn secs(at: SystemTime) -> Option<u64> {
    at.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs())
}

pub(crate) fn build(
    panes: &[Pane],
    tracker: &Tracker,
    now: SystemTime,
    mut screen: impl FnMut(u64) -> Option<String>,
) -> Built {
    let mut cards = Vec::new();
    for pane in panes {
        let Some(agent) = pane.agent.as_deref().filter(|agent| !agent.is_empty()) else { continue };
        let status = pane.status.as_deref().unwrap_or_default();
        let polled_ask =
            polled(agent, status).then(|| screen(pane.pane).and_then(|screen| ask_screen(agent, &screen))).flatten();
        let (kind, since) = match (status, polled_ask) {
            ("done", _) => (Kind::Done, tracker.started(pane, status)),
            ("failed", _) => (Kind::Failed, tracker.started(pane, status)),
            ("waiting", _) => {
                let ask = if reads_screen(agent, status) {
                    screen(pane.pane).and_then(|screen| ask_screen(agent, &screen))
                } else {
                    None
                };
                (ask.map_or(Kind::Waiting, Kind::Asks), tracker.started(pane, status))
            }
            (_, Some(ask)) => (Kind::Asks(ask), None),
            ("idle", _) => match tracker.flashes.get(&pane.pane).filter(|at| flash_live(**at, now)) {
                Some(at) => (Kind::Done, secs(*at)),
                None => continue,
            },
            _ => continue,
        };
        let place = place(pane);
        cards.push(Entry {
            tab: pane.tab,
            card: Card { pane: pane.pane, title: title(pane, &place), agent: label(agent), place, since, kind },
        });
    }
    cards.sort_by_key(|entry| (entry.card.kind.rank(), Reverse(entry.card.since)));
    let visible = panes.iter().find(|pane| pane.shown == Some(true)).map(|pane| pane.tab);
    Built { cards, visible }
}

fn place(pane: &Pane) -> String {
    match (named(pane.project_name.as_deref()), named(pane.workspace_name.as_deref())) {
        (Some(project), Some(workspace)) => format!("{project} › {workspace}"),
        (Some(only), None) | (None, Some(only)) => only.to_string(),
        (None, None) => String::new(),
    }
}

fn named(name: Option<&str>) -> Option<&str> {
    name.map(str::trim).filter(|name| !name.is_empty())
}

fn title(pane: &Pane, place: &str) -> String {
    let plain = |name: &str| {
        [pane.program.as_deref(), pane.agent.as_deref()].into_iter().flatten().any(|n| n.eq_ignore_ascii_case(name))
    };
    named(pane.tab_name.as_deref()).filter(|name| !plain(name)).map_or_else(|| place.to_string(), str::to_string)
}

pub(crate) fn still_asks(
    ask: &Ask,
    choice: usize,
    pane: Option<&Pane>,
    screen: impl FnOnce() -> Option<String>,
) -> bool {
    let Some((agent, status)) = pane.and_then(|pane| Some((pane.agent.as_deref()?, pane.status.as_deref()?))) else {
        return false;
    };
    if !reads_screen(agent, status) {
        return false;
    }
    choice < ask.choices.len() && screen().and_then(|s| ask_screen(agent, &s)).as_ref() == Some(ask)
}

#[cfg(test)]
mod tests;
