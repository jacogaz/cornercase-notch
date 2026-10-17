use crate::ask::{ALLOW, Ask, Choice, DENY};

const TITLES: [&str; 8] = [
    "Run this command?",
    "Run this command outside the sandbox?",
    "Run this MCP tool?",
    "Delete this file?",
    "Write to this file?",
    "Allow this web search?",
    "Allow this web fetch?",
    "Proceed with this edit?",
];
const ALLOWS: [&str; 4] = ["Run (once) (y)", "Proceed (y)", "Delete (y)", "Allow search (y)"];
const SKIP: &str = "Skip (esc or n)";
const NOT_ALLOWED: &str = "Not in allowlist: ";
const HIGHLIGHT: &str = " (enter)";
const QUESTION_TITLE: &str = "Clarifying Questions";
const QUESTION_FOOTER: &str = "↑/↓ option · ←/→ question · Space/Enter select · Enter to submit · Esc to skip";
const OTHER: &str = "Other";

pub(crate) fn ask_screen(screen: &str) -> Option<Ask> {
    let lines: Vec<&str> = screen.lines().map(str::trim_end).collect();
    let last = lines.iter().rposition(|line| !line.is_empty())?;
    permission(&lines[..=last]).or_else(|| question(&lines[..=last]))
}

/// One box of the screen: its content lines without the borders, and for each how many spaces are left before the
/// right border.
struct Boxed<'a> {
    top: usize,
    text: Vec<&'a str>,
    slack: Vec<usize>,
}

/// The content of a box line, " │ text   │": without the borders, and how many spaces are left before the right one.
fn cell(line: &str) -> Option<(&str, usize)> {
    let inner = line.trim_start().strip_prefix('│')?.strip_suffix('│')?.strip_prefix(' ')?;
    Some((inner.trim_end(), inner.len() - inner.trim_end().len()))
}

fn boxed<'a>(lines: &[&'a str], bottom: usize) -> Option<Boxed<'a>> {
    let framed = |line: &str, open: char, close: char| line.trim_start().starts_with(open) && line.ends_with(close);
    if !framed(lines[bottom], '└', '┘') {
        return None;
    }
    let top = (0..bottom).rev().find(|&i| framed(lines[i], '┌', '┐'))?;
    let width = lines[bottom].chars().count();
    if lines[top..=bottom].iter().any(|line| line.chars().count() != width) {
        return None;
    }
    let cells: Vec<(&str, usize)> = lines[top + 1..bottom].iter().map(|line| cell(line)).collect::<Option<_>>()?;
    Some(Boxed {
        top,
        text: cells.iter().map(|(text, _)| *text).collect(),
        slack: cells.iter().map(|(_, slack)| *slack).collect(),
    })
}

fn row(line: &str) -> Option<&str> {
    let text = line.strip_prefix(" → ").or_else(|| line.strip_prefix("   "))?;
    let text = text.strip_suffix(HIGHLIGHT).unwrap_or(text);
    text.ends_with(')').then_some(text)
}

/// A command as the screen shows it: for the card, and exactly (`None` when the screen cannot tell it whole). The
/// command box leaves one padding column before its border; the decision box's text runs into it.
struct Shown {
    detail: String,
    exact: Option<String>,
}

/// Lines wrapped by the terminal at the right border, put back together. `full` is the slack of a line that was
/// wrapped there. A line that stops shorter and is followed by another could be a newline or a space the wrap
/// swallowed, and a last line that is full could have been cut: neither is exact. A line ending in an ellipsis was
/// shortened by Cursor itself.
fn reassemble(text: &[&str], slack: &[usize], full: usize) -> Option<String> {
    let whole = slack.split_last().is_some_and(|(last, wrapped)| *last > full && wrapped.iter().all(|s| *s == full));
    let elided = text.iter().any(|line| line.ends_with('…') || line.ends_with("..."));
    (whole && !elided).then(|| text.concat())
}

/// The command in the box above the prompt, "$  <command> in <dir>"; continuation lines are indented to match.
fn command(above: &Boxed) -> Option<Shown> {
    let mut text: Vec<&str> = above.text.clone();
    text[0] = text.first()?.strip_prefix("$  ")?;
    let last = text.len() - 1;
    text[last] = text[last].rsplit_once(" in ")?.0;
    let indented: Option<Vec<&str>> =
        text.iter().enumerate().map(|(i, line)| if i == 0 { Some(*line) } else { line.strip_prefix("   ") }).collect();
    let exact = indented.and_then(|text| reassemble(&text, &above.slack, 1));
    let detail =
        exact.clone().unwrap_or_else(|| text.iter().map(|line| line.trim_start()).collect::<Vec<_>>().join("\n"));
    Some(Shown { detail, exact })
}

/// The command in the decision box's own text: "Not in allowlist: <command>", possibly wrapped.
fn said(decision: &Boxed, first_row: usize) -> Option<Shown> {
    let text = &decision.text[1..first_row];
    let first = text.first()?.strip_prefix(NOT_ALLOWED)?;
    let lines = [&[first][..], &text[1..]].concat();
    let exact = reassemble(&lines, &decision.slack[1..first_row], 0);
    Some(Shown { detail: exact.clone().unwrap_or_else(|| lines.join("\n")), exact })
}

/// The prompt is the last box on the screen, with nothing after it: a command box above it shows the command,
/// and its own first line says it again after "Not in allowlist: ". When both are there they must agree. A box above
/// is always part of the fingerprint, and one that cannot be parsed leaves only deny.
fn permission(lines: &[&str]) -> Option<Ask> {
    let decision = boxed(lines, lines.len() - 1)?;
    let title = *decision.text.first()?;
    if !TITLES.contains(&title) {
        return None;
    }
    let first_row = decision.text.iter().position(|line| line.starts_with(" → ") || line.starts_with("   "))?;
    let rows: Vec<&str> = decision.text[first_row..].iter().map(|line| row(line)).collect::<Option<_>>()?;
    if !rows.contains(&SKIP) {
        return None;
    }
    let above = decision.top.checked_sub(1).and_then(|bottom| boxed(lines, bottom));
    let boxed_command = above.as_ref().and_then(|a| Some((command(a)?, a.top)));
    let allow_line = said(&decision, first_row);
    let top = above.as_ref().map_or(decision.top, |a| a.top);
    let (agree, shown, top) = match (boxed_command, allow_line) {
        (Some((boxed, top)), Some(line)) => (boxed.exact.is_some() && boxed.exact == line.exact, boxed, top),
        (Some((boxed, top)), None) => (boxed.exact.is_some(), boxed, top),
        (None, Some(line)) => (above.is_none() && line.exact.is_some(), line, top),
        (None, None) => (false, Shown { detail: String::new(), exact: None }, top),
    };
    let mut choices = Vec::new();
    if rows.iter().any(|text| ALLOWS.contains(text)) && agree && !shown.detail.is_empty() {
        choices.push(Choice::new(ALLOW, &["y"]));
    }
    choices.push(Choice::new(DENY, &["q"]));
    Some(Ask {
        question: false,
        title: title.to_string(),
        detail: shown.detail,
        choices,
        fingerprint: lines[top..].join("\n"),
    })
}

fn question(lines: &[&str]) -> Option<Ask> {
    let bottom = lines.len() - 1;
    if !(lines[bottom].starts_with('╰') && lines[bottom].ends_with('╯')) {
        return None;
    }
    let width = lines[bottom].chars().count();
    let top = (0..bottom)
        .rev()
        .find(|&i| lines[i].starts_with("╭─ ") && lines[i].ends_with('╮') && lines[i].chars().count() == width)?;
    if !lines[top].contains(&format!(" {QUESTION_TITLE} ")) {
        return None;
    }
    let content: Vec<&str> = lines[top + 1..bottom]
        .iter()
        .map(|line| {
            line.strip_prefix('│')?.strip_suffix('│').map(|text| text.strip_prefix(' ').unwrap_or(text).trim_end())
        })
        .collect::<Option<_>>()?;
    let mut text: Vec<&str> = content.iter().copied().filter(|line| !line.is_empty()).collect();
    if text.pop()? != QUESTION_FOOTER {
        return None;
    }
    let asked = text.first()?.strip_prefix("1. ").filter(|prompt| !prompt.contains("(multi-select)"))?;
    let mut labels = Vec::new();
    for line in &text[1..] {
        let label = line.strip_prefix("› [").or_else(|| line.strip_prefix("  ["))?;
        let label = label.strip_prefix(' ').or_else(|| label.strip_prefix('x'))?.strip_prefix("] ")?;
        labels.push(label);
    }
    let total = labels.len();
    let offered: Vec<&str> = labels.iter().copied().filter(|label| !label.starts_with(OTHER)).collect();
    if offered.is_empty() || labels.iter().position(|label| label.starts_with(OTHER)).is_some_and(|at| at + 1 != total)
    {
        return None;
    }
    let choices = offered
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let mut keys = vec!["up"; total - 1];
            keys.extend(vec!["down"; i]);
            keys.push("enter");
            Choice::new(label, &keys)
        })
        .collect();
    Some(Ask {
        question: true,
        title: asked.to_string(),
        detail: String::new(),
        choices,
        fingerprint: lines[top..].join("\n"),
    })
}

#[cfg(test)]
mod tests;
