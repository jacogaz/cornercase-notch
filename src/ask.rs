use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Ask {
    pub(crate) question: bool,
    pub(crate) title: String,
    pub(crate) detail: String,
    pub(crate) choices: Vec<Choice>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Choice {
    pub(crate) label: String,
    pub(crate) key: char,
}
