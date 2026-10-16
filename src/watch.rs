use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, SystemTime};

use crate::agents::polled;
use crate::board::{self, Built, Tracker};
use crate::cornercase::{self, Event};
use crate::log;

const WHILE_WAITING: Duration = Duration::from_secs(1);

#[derive(Debug)]
pub(crate) struct Shared {
    inbox: Mutex<Inbox>,
    pub(crate) looking: AtomicBool,
    pub(crate) forward: AtomicBool,
}

#[derive(Debug, Default)]
struct Inbox {
    built: Option<Built>,
    answers: Vec<(u64, usize, bool)>,
}

impl Default for Shared {
    fn default() -> Self {
        Self { inbox: Mutex::default(), looking: AtomicBool::new(true), forward: AtomicBool::new(false) }
    }
}

impl Shared {
    pub(crate) fn take(&self) -> Option<Built> {
        self.inbox.lock().unwrap_or_else(PoisonError::into_inner).built.take()
    }

    pub(crate) fn answered(&self, pane: u64, choice: usize, sent: bool) {
        self.inbox.lock().unwrap_or_else(PoisonError::into_inner).answers.push((pane, choice, sent));
    }

    pub(crate) fn take_answers(&self) -> Vec<(u64, usize, bool)> {
        std::mem::take(&mut self.inbox.lock().unwrap_or_else(PoisonError::into_inner).answers)
    }

    fn publish(&self, built: Built) {
        self.inbox.lock().unwrap_or_else(PoisonError::into_inner).built = Some(built);
    }

    pub(crate) fn looking(&self) -> bool {
        self.looking.load(Ordering::Relaxed)
    }
}

pub(crate) fn start(shared: Arc<Shared>) {
    thread::spawn(move || {
        follow(&shared);
        std::process::exit(1);
    });
}

fn follow(shared: &Shared) {
    let events = match cornercase::events() {
        Ok(events) => events,
        Err(e) => return log::line(&format!("could not follow the events: {e}")),
    };
    let (mut tracker, mut last, mut failed) = (Tracker::default(), None::<Built>, None::<String>);
    let mut polling = false;
    let mut pending: Vec<Event> = Vec::new();
    loop {
        let built = rebuild(shared, &mut tracker, &pending, &mut failed);
        pending.clear();
        if let Some((_, watching)) = &built {
            polling = *watching;
        }
        if let Some(built) = built.map(|(built, _)| built).filter(|built| last.as_ref() != Some(built)) {
            last = Some(built.clone());
            shared.publish(built);
        }
        let busy = polling || last.as_ref().is_some_and(Built::waiting) || tracker.flashing(SystemTime::now());
        let next = if busy {
            events.changes.recv_timeout(WHILE_WAITING)
        } else {
            events.changes.recv().map_err(|_| RecvTimeoutError::Disconnected)
        };
        match next {
            Ok(event) => {
                pending.push(event);
                pending.extend(events.changes.try_iter());
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return log::line("the events ended; the server stopped"),
        }
    }
}

/// The new board, and whether some pane has to be read on a timer because its agent raises no event for a prompt.
fn rebuild(
    shared: &Shared,
    tracker: &mut Tracker,
    events: &[Event],
    failed: &mut Option<String>,
) -> Option<(Built, bool)> {
    let panes = match cornercase::panes() {
        Ok(panes) => panes,
        Err(e) => {
            report(failed, Some(format!("could not read the panes: {e}")));
            return None;
        }
    };
    let now = SystemTime::now();
    tracker.retain(&panes);
    for event in events {
        let shown = panes.iter().any(|pane| Some(pane.pane) == event.pane && pane.shown == Some(true));
        tracker.observe(event, shown, shared.looking(), now);
    }
    let mut error = None;
    let built = board::build(&panes, tracker, now, |pane| {
        cornercase::screen(pane).map_err(|e| error = Some(format!("could not read pane {pane}: {e}"))).ok()
    });
    report(failed, error);
    let watching = panes
        .iter()
        .any(|pane| polled(pane.agent.as_deref().unwrap_or_default(), pane.status.as_deref().unwrap_or_default()));
    Some((built, watching))
}

fn report(last: &mut Option<String>, now: Option<String>) {
    if let Some(message) = now.as_ref().filter(|message| last.as_ref() != Some(*message)) {
        log::line(message);
    }
    *last = now;
}
