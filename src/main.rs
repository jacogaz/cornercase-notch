#![allow(dead_code, reason = "the panel and the plugin protocol that use these come in later steps")]

mod agents;
mod ask;
mod notch;
#[cfg(test)]
mod test_util;

fn main() {
    eprintln!(
        "usage: cornercase-notch\n\nThe notch panel for cornercase. It is started by cornercase; there is nothing to run by hand yet."
    );
}
