use super::super::numbered::{choice, options};
use crate::ask::{ALLOW, Ask, DENY};

const TITLE: &str = "  Would you like to run the following command?";
const FOOTER: &str = "  Press enter to confirm or esc to cancel";
const FOOTER_THREAD: &str = " or o to open thread";
const CURSOR: char = '›';
const COMMAND: &str = "  $ ";
const CONTINUED: &str = "    ";
const ELIDED: &str = "  [… ";
const HEADERS: [&str; 4] = ["  Environment: ", "  Reason: ", "  Permission rule: ", "  Thread: "];
const ALLOWS: [&str; 2] = ["Yes, proceed", "Yes, just this once"];
const DENIES: &str = "No, and tell Codex what to do differently";
const COMMAND_TITLE: &str = "Run command";

pub(crate) fn ask_screen(screen: &str) -> Option<Ask> {
    let lines: Vec<&str> = screen.lines().map(str::trim_end).collect();
    let footer = lines.iter().rposition(|line| !line.is_empty())?;
    if !(lines[footer] == FOOTER || lines[footer].strip_prefix(FOOTER).is_some_and(|rest| rest == FOOTER_THREAD)) {
        return None;
    }
    let title = lines[..footer].iter().rposition(|line| *line == TITLE)?;
    let end = (0..footer).rev().find(|&i| !lines[i].is_empty())?;
    let start = (title + 1..=end)
        .rev()
        .find(|&i| options(&lines[i..=end], 0, false, CURSOR).is_some_and(|o| o[0].number == 1))?;
    let options = options(&lines[start..=end], 0, false, CURSOR)?;
    let (command, elided) = command(&lines[title + 1..start])?;
    let allow = options.iter().find(|o| ALLOWS.iter().any(|label| o.label.starts_with(label)));
    let deny = options.iter().find(|o| o.label.starts_with(DENIES))?;
    let fingerprint = lines[title..=footer].join("\n");
    let mut choices = Vec::new();
    if let Some(allow) = allow.filter(|_| !elided) {
        choices.push(choice(ALLOW, allow.number)?);
    }
    choices.push(choice(DENY, deny.number)?);
    allow?;
    Some(Ask { question: false, title: COMMAND_TITLE.into(), detail: command, choices, fingerprint })
}

fn command(lines: &[&str]) -> Option<(String, bool)> {
    let (mut command, mut elided, mut in_command) = (Vec::new(), false, false);
    for line in lines {
        if line.is_empty() {
            in_command = false;
        } else if let Some(first) = line.strip_prefix(COMMAND) {
            command.push(first);
            in_command = true;
        } else if in_command && let Some(next) = line.strip_prefix(CONTINUED) {
            command.push(next);
        } else if line.starts_with(ELIDED) {
            elided = true;
            in_command = false;
        } else if HEADERS.iter().any(|header| line.starts_with(header)) {
            in_command = false;
        } else {
            return None;
        }
    }
    (!command.is_empty() || elided).then(|| (command.join("\n"), elided))
}

#[cfg(test)]
mod tests;
