use std::thread;
use std::time::Duration;

use crate::agents::{backs_out, checks_after_press};
use crate::board::still_asks;
use crate::cornercase;
use crate::log;
use crate::notch::Action;

const SETTLE: Duration = Duration::from_millis(400);

pub(crate) struct Report {
    pub(crate) said: String,
    pub(crate) sent: bool,
}

pub(crate) fn spawn(
    action: Action,
    forward: impl Fn() + Send + 'static,
    done: impl FnOnce(&Action, &Report) + Send + 'static,
) {
    thread::spawn(move || {
        let report = run(&action, &forward);
        done(&action, &report);
    });
}

pub(crate) fn run(action: &Action, forward: &dyn Fn()) -> Report {
    match action {
        Action::Open(pane) => Report { said: open(*pane, forward), sent: false },
        Action::Answer { pane, ask, choice } => {
            let panes = cornercase::panes().map_err(|e| log::line(&format!("could not read the panes: {e}")));
            let found = panes.iter().flatten().find(|p| p.pane == *pane);
            if !still_asks(ask, *choice, found, || cornercase::screen(*pane).ok()) {
                let said = format!("pane {pane} no longer asks that; showing it instead");
                log::line(&said);
                return Report { said: format!("{said}; {}", open(*pane, forward)), sent: false };
            }
            let agent = found.and_then(|p| p.agent.clone()).unwrap_or_default();
            let keys = &ask.choices[*choice].keys;
            let kind = if ask.question { "question" } else { "permission" };
            if let Err(e) = cornercase::keys(*pane, keys) {
                log::line(&format!("could not press {} in pane {pane}: {e}", keys.join(" ")));
                return Report {
                    said: format!("could not press the keys in pane {pane}; {}", open(*pane, forward)),
                    sent: false,
                };
            }
            if let Some(said) = back_out(*pane, &agent, forward) {
                return Report { said, sent: false };
            }
            let said = format!("answered pane {pane} with {} ({kind})", keys.join(" "));
            log::line(&said);
            Report { said, sent: true }
        }
    }
}

/// Some agents can answer a key with a stage the user did not choose. If the screen shows one, press
/// Escape once, show the pane, and say that nothing was answered.
fn back_out(pane: u64, agent: &str, forward: &dyn Fn()) -> Option<String> {
    if !checks_after_press(agent) {
        return None;
    }
    thread::sleep(SETTLE);
    let screen = cornercase::screen(pane).ok()?;
    if !backs_out(agent, &screen) {
        return None;
    }
    let pressed = cornercase::keys(pane, &["esc".to_string()]);
    let said =
        format!("pane {pane} went to a stage that was not chosen; pressed esc ({}); showing it", pressed.is_ok());
    log::line(&said);
    open(pane, forward);
    Some(said)
}

fn open(pane: u64, forward: &dyn Fn()) -> String {
    let said = match cornercase::focus(pane) {
        Ok(()) => format!("showed pane {pane}"),
        Err(e) => format!("could not show pane {pane}: {e}"),
    };
    log::line(&said);
    forward();
    said
}
