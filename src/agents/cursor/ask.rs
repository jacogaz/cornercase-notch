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

/// One box of the screen: its content lines without the borders, and whether a line reaches the right border.
struct Boxed<'a> {
    top: usize,
    text: Vec<&'a str>,
    cut: bool,
}

/// The content of a box line, " │ text   │": without the borders, and whether a space is left before the right one.
fn cell(line: &str) -> Option<(&str, bool)> {
    let inner = line.trim_start().strip_prefix('│')?.strip_suffix('│')?.strip_prefix(' ')?;
    Some((inner.trim_end(), inner.ends_with(' ')))
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
    let cells: Vec<(&str, bool)> = lines[top + 1..bottom].iter().map(|line| cell(line)).collect::<Option<_>>()?;
    Some(Boxed { top, cut: cells.iter().any(|(_, room)| !room), text: cells.iter().map(|(text, _)| *text).collect() })
}

fn row(line: &str) -> Option<&str> {
    let text = line.strip_prefix(" → ").or_else(|| line.strip_prefix("   "))?;
    let text = text.strip_suffix(HIGHLIGHT).unwrap_or(text);
    text.ends_with(')').then_some(text)
}

/// The command in the box above the prompt, "$  <command> in <dir>".
fn command(above: &Boxed) -> Option<String> {
    let mut text: Vec<&str> = above.text.clone();
    text[0] = text.first()?.strip_prefix("$  ")?;
    let last = text.len() - 1;
    text[last] = text[last].rsplit_once(" in ")?.0;
    Some(text.join("\n"))
}

/// The prompt is the last box on the screen, with nothing after it: a command box above it shows the command,
/// and its own first line says it again after "Not in allowlist: ".
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
    let (shown, cut, top) = if let Some((command, above)) = above.as_ref().and_then(|a| Some((command(a)?, a))) {
        (command, above.cut, above.top)
    } else {
        let said = decision.text[1..first_row].join("\n");
        (said.strip_prefix(NOT_ALLOWED).unwrap_or_default().to_string(), decision.cut, decision.top)
    };
    let mut choices = Vec::new();
    if rows.iter().any(|text| ALLOWS.contains(text)) && !shown.is_empty() && !cut {
        choices.push(Choice::new(ALLOW, &["y"]));
    }
    choices.push(Choice::new(DENY, &["q"]));
    Some(Ask {
        question: false,
        title: title.to_string(),
        detail: shown,
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
