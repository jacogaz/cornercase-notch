use std::thread;

use crate::board::still_asks;
use crate::cornercase;
use crate::log;
use crate::notch::Action;

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
        Action::Answer { pane, ask, key } => {
            let panes = cornercase::panes().map_err(|e| log::line(&format!("could not read the panes: {e}")));
            let found = panes.iter().flatten().find(|p| p.pane == *pane);
            if !still_asks(ask, *key, found, || cornercase::screen(*pane).ok()) {
                let said = format!("pane {pane} no longer asks that; showing it instead");
                log::line(&said);
                return Report { said: format!("{said}; {}", open(*pane, forward)), sent: false };
            }
            match cornercase::keys(*pane, *key) {
                Ok(()) => {
                    let kind = if ask.question { "question" } else { "permission" };
                    let said = format!("answered pane {pane} with {key} ({kind})");
                    log::line(&said);
                    Report { said, sent: true }
                }
                Err(e) => {
                    log::line(&format!("could not press {key} in pane {pane}: {e}"));
                    Report {
                        said: format!("could not press {key} in pane {pane}; {}", open(*pane, forward)),
                        sent: false,
                    }
                }
            }
        }
    }
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
