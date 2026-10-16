mod act;
mod agents;
mod ask;
mod board;
mod cornercase;
mod lock;
mod log;
mod looking;
mod notch;
#[cfg(test)]
mod test_util;
mod watch;

use std::process::ExitCode;

use serde::Serialize;

use crate::board::{Built, Tracker};
use crate::notch::Action;

const USAGE: &str = "usage: cornercase-notch <panel | board | act '<action as JSON>'>

  panel   show the notch (macOS); cornercase starts it as the plugin's window process
  board   print the board once as JSON, with each card's tab, the visible tab and whether you look
  act     run one action, as a click would, and print what it did";

#[derive(Serialize)]
struct Report {
    #[serde(flatten)]
    built: Built,
    looking: bool,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["panel"] => panel(),
        ["board"] => board(),
        ["act", action] => act(action),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            log::line(&message);
            ExitCode::FAILURE
        }
    }
}

fn board() -> Result<(), String> {
    let panes = cornercase::panes().map_err(|e| format!("could not read the panes: {e}"))?;
    let built =
        board::build(&panes, &Tracker::default(), std::time::SystemTime::now(), |pane| cornercase::screen(pane).ok());
    let looking = looking::looking(looking::terminal());
    let json = serde_json::to_string(&Report { built, looking }).map_err(|e| e.to_string())?;
    println!("{json}");
    Ok(())
}

fn act(action: &str) -> Result<(), String> {
    let action: Action = serde_json::from_str(action).map_err(|e| format!("not an action: {e}"))?;
    println!("{}", act::run(&action, &|| looking::bring_forward(looking::terminal())).said);
    Ok(())
}

#[cfg(target_os = "macos")]
fn panel() -> Result<(), String> {
    let _lock = lock::acquire(&lock::path()).map_err(|e| format!("could not take the notch lock: {e}"))?;
    let terminal = looking::terminal();
    let shared = std::sync::Arc::new(watch::Shared::default());
    watch::start(std::sync::Arc::clone(&shared));
    notch::panel::run(shared, terminal);
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn panel() -> Result<(), String> {
    Err("the notch is only on macOS".into())
}
