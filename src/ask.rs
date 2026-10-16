use serde::{Deserialize, Serialize};

pub(crate) const ALLOW: &str = "allow";
pub(crate) const DENY: &str = "deny";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Ask {
    pub(crate) question: bool,
    pub(crate) title: String,
    pub(crate) detail: String,
    pub(crate) choices: Vec<Choice>,
    #[serde(default)]
    pub(crate) fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Choice {
    pub(crate) label: String,
    pub(crate) key: char,
}
