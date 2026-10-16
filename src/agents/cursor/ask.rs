use crate::ask::{ALLOW, Ask, Choice, DENY};

const RULE: char = '─';
const RULE_MIN: usize = 8;
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
const DENY_HINTS: [&str; 2] = [" (esc or n)", " (n)"];
const HIGHLIGHT: &str = " (enter)";
const QUESTION_TITLE: &str = "Clarifying Questions";
const QUESTION_FOOTER: &str = "↑/↓ option · ←/→ question · Space/Enter select · Enter to submit · Esc to skip";
const OTHER: &str = "Other";

pub(crate) fn ask_screen(screen: &str) -> Option<Ask> {
    let lines: Vec<&str> = screen.lines().map(str::trim_end).collect();
    let last = lines.iter().rposition(|line| !line.is_empty())?;
    permission(&lines[..=last]).or_else(|| question(&lines[..=last]))
}

fn rule(line: &str) -> bool {
    line.chars().count() >= RULE_MIN && line.chars().all(|c| c == RULE)
}

fn row(line: &str) -> Option<&str> {
    let text = line.strip_prefix("→ ").or_else(|| line.strip_prefix("  "))?;
    let text = text.strip_suffix(HIGHLIGHT).unwrap_or(text);
    text.ends_with(')').then_some(text)
}

fn permission(lines: &[&str]) -> Option<Ask> {
    let title = lines.iter().rposition(|line| TITLES.contains(line))?;
    if title == 0 || !rule(lines[title - 1]) {
        return None;
    }
    let mut rows = Vec::new();
    for (i, line) in lines[title + 1..].iter().enumerate() {
        match row(line) {
            Some(text) => rows.push(text),
            None if i == 0 && !line.starts_with("→ ") && !line.starts_with("  ") => {}
            None => return None,
        }
    }
    let mut choices = Vec::new();
    if rows.iter().any(|text| ALLOWS.contains(text)) {
        choices.push(Choice::new(ALLOW, &["y"]));
    }
    if rows.iter().any(|text| DENY_HINTS.iter().any(|hint| text.ends_with(hint))) {
        choices.push(Choice::new(DENY, &["n"]));
    }
    if choices.is_empty() {
        return None;
    }
    let (detail, top) = banner(lines, title - 1);
    Some(Ask {
        question: false,
        title: lines[title].to_string(),
        detail,
        choices,
        fingerprint: lines[top..].join("\n"),
    })
}

/// What the box above the prompt says, and where the prompt's region starts.
fn banner(lines: &[&str], rule_at: usize) -> (String, usize) {
    let framed = |line: &str, open: char, close: char| line.starts_with(open) && line.ends_with(close);
    let Some(bottom) = rule_at.checked_sub(1).filter(|&i| framed(lines[i], '└', '┘')) else {
        return (String::new(), rule_at);
    };
    let Some(top) = (0..bottom).rev().find(|&i| framed(lines[i], '┌', '┐')) else {
        return (String::new(), rule_at);
    };
    let text: Option<Vec<&str>> = lines[top + 1..bottom]
        .iter()
        .map(|line| {
            line.strip_prefix('│')?.strip_suffix('│').map(|text| text.strip_prefix(' ').unwrap_or(text).trim_end())
        })
        .collect();
    match text {
        Some(text) => (text.join("\n"), top),
        None => (String::new(), rule_at),
    }
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
