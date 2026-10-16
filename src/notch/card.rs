use std::time::{Duration, Instant};

use super::rinco::Mood;
use super::{Action, Card, Kind};
use crate::ask::ALLOW;

pub(crate) const COMMAND_LINES: usize = 3;
const MAX_COMMAND: usize = 100;
const DONE_FOR: Duration = Duration::from_millis(3500);
const EXPAND_FOR: Duration = Duration::from_secs(4);
const GREEN: u32 = 0x58_e6_a0;
const YELLOW: u32 = 0xf5_d7_6e;
const PURPLE: u32 = 0xb2_94_ff;
const CYAN: u32 = 0x45_d9_e8;
const PINK: u32 = 0xff_8a_a3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Press {
    Open(u64),
    Answer(Action),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Button {
    pub(crate) label: String,
    pub(crate) press: Press,
    pub(crate) fill: u32,
    pub(crate) text: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Outcome {
    Picked(String),
    Running(String),
    Denied(String),
}

impl Outcome {
    pub(crate) fn line(&self) -> (String, u32) {
        match self {
            Self::Picked(option) => (format!("✓ You picked {option}"), GREEN),
            Self::Running(command) => (format!("◐ Running {command}…"), CYAN),
            Self::Denied(agent) => (format!("✕ Denied. {agent} will try another way."), PINK),
        }
    }

    pub(crate) fn mood(&self) -> Mood {
        match self {
            Self::Picked(_) => Mood::Done,
            Self::Running(_) => Mood::Working,
            Self::Denied(_) => Mood::Error,
        }
    }

    pub(crate) fn monospaced(&self) -> bool {
        matches!(self, Self::Running(_))
    }
}

pub(crate) fn mood(card: &Card) -> Mood {
    match &card.kind {
        Kind::Asks(ask) if ask.question => Mood::Question,
        Kind::Asks(_) | Kind::Waiting => Mood::Needs,
        Kind::Failed => Mood::Error,
        Kind::Done => Mood::Done,
    }
}

pub(crate) fn subtitle(card: &Card) -> String {
    if card.title == card.place { card.agent.clone() } else { format!("{} · {}", card.agent, card.place) }
}

fn name(card: &Card) -> String {
    if card.title == card.place { format!("{} · {}", card.agent, card.place) } else { card.title.clone() }
}

pub(crate) fn summary(card: &Card) -> String {
    match &card.kind {
        Kind::Asks(ask) if ask.question || ask.detail.is_empty() => format!("{} · {}", name(card), ask.title),
        Kind::Asks(ask) => format!("{} · {}", name(card), one_line(&ask.detail)),
        Kind::Waiting => format!("{} needs you", name(card)),
        Kind::Failed => format!("{} failed", name(card)),
        Kind::Done => format!("{} finished", name(card)),
    }
}

pub(crate) fn more(count: usize) -> Option<String> {
    (count > 1).then(|| format!("+{}", count - 1))
}

pub(crate) fn arrived(before: &[Card], after: &[Card]) -> bool {
    after.iter().any(|card| card.kind != Kind::Done && !before.contains(card))
}

pub(crate) fn lasts(card: &Card) -> Option<Duration> {
    (card.kind == Kind::Done).then_some(DONE_FOR)
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Expand {
    hovered: bool,
    until: Option<Instant>,
}

impl Expand {
    pub(crate) fn hover(&mut self, hovered: bool, now: Instant) {
        if self.hovered && !hovered {
            self.until = Some(now + EXPAND_FOR);
        }
        self.hovered = hovered;
    }

    pub(crate) fn arrive(&mut self, now: Instant) {
        self.until = Some(now + EXPAND_FOR);
    }

    pub(crate) fn at(&self, now: Instant) -> bool {
        self.hovered || self.until.is_some_and(|until| now < until)
    }
}

pub(crate) fn status(card: &Card) -> (String, u32) {
    match &card.kind {
        Kind::Done => ("Finished".into(), GREEN),
        Kind::Failed => ("Failed".into(), PINK),
        Kind::Waiting => ("Needs you".into(), YELLOW),
        Kind::Asks(ask) if ask.question => (ask.title.clone(), PURPLE),
        Kind::Asks(ask) if ask.detail.is_empty() => (format!("Wants: {}", ask.title), YELLOW),
        Kind::Asks(_) => ("Wants to run:".into(), YELLOW),
    }
}

pub(crate) fn command(card: &Card) -> Option<&str> {
    match &card.kind {
        Kind::Asks(ask) if !ask.question && !ask.detail.is_empty() => Some(&ask.detail),
        _ => None,
    }
}

pub(crate) fn command_fits(command: &str) -> bool {
    !command.is_empty() && command.lines().count() <= COMMAND_LINES && command.chars().count() <= MAX_COMMAND
}

fn one_line(command: &str) -> String {
    command.replace('\n', " ⏎ ")
}

pub(crate) fn outcome(card: &Card, key: char) -> Option<Outcome> {
    let Kind::Asks(ask) = &card.kind else { return None };
    let choice = ask.choices.iter().find(|c| c.key == key)?;
    Some(if ask.question {
        Outcome::Picked(choice.label.clone())
    } else if choice.label == ALLOW {
        Outcome::Running(if ask.detail.is_empty() { ask.title.clone() } else { one_line(&ask.detail) })
    } else {
        Outcome::Denied(card.agent.clone())
    })
}

pub(crate) fn buttons(card: &Card, fits: bool) -> Vec<Button> {
    let button = |label: &str, press, (fill, text)| Button { label: label.to_string(), press, fill, text };
    let plain = (0x2e_2d_33, 0xff_ff_ff);
    let open = button("open", Press::Open(card.pane), plain);
    match &card.kind {
        Kind::Done | Kind::Failed | Kind::Waiting => vec![open],
        Kind::Asks(ask) => ask
            .choices
            .iter()
            .filter(|choice| ask.question || choice.label != ALLOW || (fits && command_fits(&ask.detail)))
            .map(|choice| {
                let action = Action::Answer { pane: card.pane, ask: ask.clone(), key: choice.key };
                let colours = match (ask.question, choice.label == ALLOW) {
                    (true, _) => (0x2a_21_50, 0xc9_b8_ff),
                    (false, true) => (0x1d_3a_2c, GREEN),
                    (false, false) => (0x45_20_2b, PINK),
                };
                button(&choice.label, Press::Answer(action), colours)
            })
            .chain([open])
            .collect(),
    }
}

pub(crate) fn ago(secs: u64) -> String {
    match secs {
        0..60 => "now".into(),
        60..3_600 => format!("{}m", secs / 60),
        3_600..86_400 => format!("{}h", secs / 3_600),
        _ => format!("{}d", secs / 86_400),
    }
}

#[cfg(test)]
mod tests;
