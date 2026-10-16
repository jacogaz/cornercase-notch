use std::ffi::OsString;
use std::io::{self, BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[expect(clippy::struct_field_names, reason = "the fields are the names the commands print")]
pub(crate) struct Pane {
    pub(crate) pane: u64,
    pub(crate) tab: u64,
    #[serde(default, deserialize_with = "lenient")]
    pub(crate) agent: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    pub(crate) status: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    pub(crate) program: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    pub(crate) tab_name: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    pub(crate) workspace_name: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    pub(crate) project_name: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    pub(crate) shown: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[expect(clippy::struct_field_names, reason = "the fields are the names the commands print")]
pub(crate) struct Event {
    #[serde(default, deserialize_with = "lenient")]
    pub(crate) event: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    pub(crate) pane: Option<u64>,
    #[serde(default, deserialize_with = "lenient")]
    pub(crate) from: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    pub(crate) to: Option<String>,
}

fn lenient<'de, D: Deserializer<'de>, T: DeserializeOwned>(deserializer: D) -> Result<Option<T>, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).ok())
}

pub(crate) fn parse_panes(json: &str) -> io::Result<Vec<Pane>> {
    #[derive(Deserialize)]
    struct Status {
        #[serde(default)]
        panes: Vec<serde_json::Value>,
    }
    let status: Status = serde_json::from_str(json).map_err(io::Error::other)?;
    Ok(status.panes.into_iter().filter_map(|pane| serde_json::from_value(pane).ok()).collect())
}

pub(crate) fn parse_event(line: &str) -> Option<Event> {
    serde_json::from_str(line).ok()
}

fn command() -> Command {
    let bin = std::env::var_os("CORNERCASE_BIN").filter(|bin| !bin.is_empty());
    Command::new(bin.unwrap_or_else(|| OsString::from("cornercase")))
}

fn run(args: &[&str]) -> io::Result<String> {
    let output = command().args(args).stdin(Stdio::null()).output()?;
    if !output.status.success() {
        let why = String::from_utf8_lossy(&output.stderr);
        let why = why.lines().next().unwrap_or("").trim();
        return Err(io::Error::other(format!("`cornercase {}` failed ({}): {why}", args[0], output.status)));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub(crate) fn panes() -> io::Result<Vec<Pane>> {
    parse_panes(&run(&["status", "--panes", "--json"])?)
}

pub(crate) fn screen(pane: u64) -> io::Result<String> {
    run(&["read", "--pane", &pane.to_string()])
}

pub(crate) fn keys(pane: u64, keys: &[String]) -> io::Result<()> {
    let pane = pane.to_string();
    let mut args = vec!["keys", "--pane", &pane];
    args.extend(keys.iter().map(String::as_str));
    run(&args).map(drop)
}

pub(crate) fn focus(pane: u64) -> io::Result<()> {
    run(&["focus", "--pane", &pane.to_string()]).map(drop)
}

pub(crate) struct Events {
    child: Child,
    pub(crate) changes: Receiver<Event>,
}

impl Drop for Events {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub(crate) fn events() -> io::Result<Events> {
    let mut child = command()
        .args(["events", "--json"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        return Err(io::Error::other("`cornercase events` started without its output"));
    };
    let (sender, changes) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { return };
            if sender.send(parse_event(&line).unwrap_or_default()).is_err() {
                return;
            }
        }
    });
    Ok(Events { child, changes })
}

#[cfg(test)]
mod tests;
