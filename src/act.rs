use std::thread;

use crate::board::still_asks;
use crate::cornercase;
use crate::log;
use crate::notch::Action;

pub(crate) fn spawn(action: Action, forward: impl Fn() + Send + 'static) {
    thread::spawn(move || {
        run(&action, &forward);
    });
}

pub(crate) fn run(action: &Action, forward: &dyn Fn()) -> String {
    match action {
        Action::Open(pane) => open(*pane, forward),
        Action::Answer { pane, ask, key } => {
            let panes = cornercase::panes().map_err(|e| log::line(&format!("could not read the panes: {e}")));
            let found = panes.iter().flatten().find(|p| p.pane == *pane);
            if !still_asks(ask, *key, found, || cornercase::screen(*pane).ok()) {
                let said = format!("pane {pane} no longer asks that; showing it instead");
                log::line(&said);
                return format!("{said}; {}", open(*pane, forward));
            }
            match cornercase::keys(*pane, *key) {
                Ok(()) => {
                    let said = format!(
                        "answered pane {pane} with {key} ({})",
                        if ask.question { "question" } else { "permission" }
                    );
                    log::line(&said);
                    said
                }
                Err(e) => {
                    log::line(&format!("could not press {key} in pane {pane}: {e}"));
                    format!("could not press {key} in pane {pane}; {}", open(*pane, forward))
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
