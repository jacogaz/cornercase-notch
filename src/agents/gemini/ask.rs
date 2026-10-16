use super::super::numbered::options;
use crate::ask::{ALLOW, Ask, Choice, DENY};

const CURSOR: char = '●';
const HEADER: &str = "? ";
const QUESTIONS: [&str; 3] = ["Allow execution of ", "Apply this change?", "Do you want to proceed?"];
const ALLOW_ONCE: &str = "Allow once";
const DENY_HINT: &str = "(esc)";
const HIDDEN: &str = "hidden (Ctrl+O)";
const QUESTION_FOOTER: &str = "Enter to select · ↑/↓ to navigate · Esc to cancel";
const MAX_ROWS: usize = 9;

pub(crate) fn ask_screen(screen: &str) -> Option<Ask> {
    let lines: Vec<&str> = screen.lines().map(str::trim_end).collect();
    let last = lines.iter().rposition(|line| !line.is_empty())?;
    match unbox(&lines, last) {
        Some(content) => {
            let content: Vec<&str> = content.iter().map(String::as_str).collect();
            permission(&content).or_else(|| question(&content))
        }
        None => question(&lines[..=last]),
    }
}

fn framed(line: &str, open: char, close: char) -> bool {
    line.starts_with(open) && line.ends_with(close)
}

/// The lines inside the outer rounded box when its bottom border is the last line on screen.
fn unbox(lines: &[&str], last: usize) -> Option<Vec<String>> {
    let bottom = lines[last];
    if !framed(bottom, '╰', '╯') {
        return None;
    }
    let width = bottom.chars().count();
    let top = lines[..last].iter().rposition(|line| framed(line, '╭', '╮') && line.chars().count() == width)?;
    lines[top + 1..last].iter().map(|line| inside(line)).collect()
}

fn inside(line: &str) -> Option<String> {
    let text = line.strip_prefix('│')?.strip_suffix('│')?;
    Some(text.strip_prefix(' ').unwrap_or(text).trim_end().to_string())
}

fn permission(content: &[&str]) -> Option<Ask> {
    content.first().filter(|line| line.starts_with(HEADER))?;
    let top = content.iter().position(|line| framed(line, '╭', '╮'))?;
    let bottom = top + content[top..].iter().position(|line| framed(line, '╰', '╯'))?;
    let shown: Vec<String> = content[top + 1..bottom].iter().map(|line| inside(line)).collect::<Option<_>>()?;
    let elided = shown.iter().any(|line| line.trim_start().starts_with("...") && line.contains(HIDDEN));
    let asked = (bottom + 1..content.len()).find(|&i| !content[i].is_empty())?;
    if !content[bottom + 1..asked].iter().all(|line| line.is_empty())
        || !QUESTIONS.iter().any(|question| content[asked].starts_with(question))
        || !content[asked].ends_with('?')
    {
        return None;
    }
    let end = (asked + 1..content.len()).rev().find(|&i| !content[i].is_empty())?;
    let first = (asked + 1..=end).find(|&i| !content[i].is_empty())?;
    if !content[asked + 1..first].iter().all(|line| line.is_empty()) {
        return None;
    }
    let rows = options(&content[first..=end], 0, false, CURSOR)?;
    if rows.len() > MAX_ROWS {
        return None;
    }
    let allow = rows.first().filter(|row| row.label == ALLOW_ONCE)?;
    let deny = rows.iter().find(|row| row.label.ends_with(DENY_HINT))?;
    let mut choices = Vec::new();
    if !elided {
        choices.push(digit(ALLOW, allow.number)?);
    }
    choices.push(digit(DENY, deny.number)?);
    Some(Ask {
        question: false,
        title: content[asked].to_string(),
        detail: shown.join("\n"),
        choices,
        fingerprint: content.join("\n"),
    })
}

fn digit(label: &str, number: u32) -> Option<Choice> {
    Some(Choice::new(label, &[&char::from_digit(number, 10)?.to_string()]))
}

fn question(lines: &[&str]) -> Option<Ask> {
    let footer = lines.iter().rposition(|line| !line.is_empty()).filter(|&i| lines[i] == QUESTION_FOOTER)?;
    let end = (0..footer).rev().find(|&i| !lines[i].is_empty())?;
    let blank = (0..end).rev().find(|&i| lines[i].is_empty())?;
    let rows = options(&lines[blank + 1..=end], 0, true, CURSOR)?;
    let asked = (0..blank).rev().find(|&i| !lines[i].is_empty())?;
    if !lines[asked + 1..blank].is_empty() || asked.checked_sub(1).is_some_and(|above| !lines[above].is_empty()) {
        return None;
    }
    let (_custom, offered) = rows.split_last()?;
    if offered.is_empty() || rows.len() > MAX_ROWS || rows.iter().any(|row| row.label.contains('[')) {
        return None;
    }
    let choices = offered.iter().map(|row| digit(&row.label, row.number)).collect::<Option<Vec<_>>>()?;
    Some(Ask {
        question: true,
        title: lines[asked].to_string(),
        detail: String::new(),
        choices,
        fingerprint: lines[asked..=footer].join("\n"),
    })
}

#[cfg(test)]
mod tests;
