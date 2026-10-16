use serde::{Deserialize, Serialize};

use crate::ask::Ask;

#[cfg(any(target_os = "macos", test))]
pub(crate) mod card;
#[cfg(any(target_os = "macos", test))]
pub(crate) mod motion;
#[cfg(target_os = "macos")]
pub(crate) mod panel;
#[cfg(any(target_os = "macos", test))]
pub(crate) mod rinco;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Board {
    pub(crate) cards: Vec<Card>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Card {
    pub(crate) pane: u64,
    pub(crate) title: String,
    pub(crate) place: String,
    pub(crate) since: Option<u64>,
    pub(crate) kind: Kind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Kind {
    Done,
    Waiting,
    Asks(Ask),
}

impl Kind {
    pub(crate) fn rank(&self) -> u8 {
        match self {
            Self::Asks(_) => 0,
            Self::Waiting => 1,
            Self::Done => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Action {
    Open(u64),
    Answer { pane: u64, ask: Ask, key: char },
}

#[cfg(test)]
mod tests;
