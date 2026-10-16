use super::super::numbered::{choice, options};
use crate::ask::{ALLOW, Ask, DENY};

const RULE: char = '─';
const DASH: char = '╌';
const RULE_MIN: usize = 8;
const CURSOR: char = '❯';
const PROCEED: &str = " Do you want to ";
const PERMISSION_FOOTER: &str = " Esc to cancel";
const QUESTION_FOOTER: &str = "Enter to select · ↑/↓ to navigate · Esc to cancel";
const HEADER: &str = " ☐ ";
const FREE_TEXT: &str = "Type something.";
const CHECKBOX: &str = "[ ]";
const YES: &str = "Yes";
const NO: &str = "No";
const MAX_QUESTION_LINES: usize = 4;

pub(crate) fn ask_screen(screen: &str) -> Option<Ask> {
    let lines: Vec<&str> = screen.lines().map(str::trim_end).collect();
    permission(&lines).or_else(|| question(&lines))
}

fn permission(lines: &[&str]) -> Option<Ask> {
    let footer = last_line(lines).filter(|&i| lines[i].starts_with(PERMISSION_FOOTER))?;
    let end = last_filled(lines, footer)?;
    let start = (0..=end).rev().find(|&i| lines[i].starts_with(PROCEED) && lines[i].ends_with('?'))?;
    let options = options(&lines[start + 1..=end], 1, false, CURSOR)?;
    let first = options.first().filter(|o| o.label == YES)?;
    let no = options.iter().find(|o| o.label == NO || o.label.starts_with("No,"))?;
    let top = (0..start).rev().find(|&i| rule(lines[i], RULE))?;
    let title = lines.get(top + 1).map(|line| line.trim()).filter(|t| !t.is_empty() && top + 1 < start)?;
    let dashes: Vec<usize> = (top + 1..start).filter(|&i| rule(lines[i], DASH)).collect();
    let detail = match dashes.as_slice() {
        [open, close, ..] => {
            lines[open + 1..*close].iter().map(|l| l.strip_prefix(' ').unwrap_or(l)).collect::<Vec<_>>().join("\n")
        }
        _ => String::new(),
    };
    let choices = vec![choice(ALLOW, first.number)?, choice(DENY, no.number)?];
    let fingerprint = lines[top..=footer].join("\n");
    Some(Ask { question: false, title: title.to_string(), detail, choices, fingerprint })
}

fn question(lines: &[&str]) -> Option<Ask> {
    let footer = last_line(lines).filter(|&i| lines[i] == QUESTION_FOOTER)?;
    let below = (0..footer).rev().find(|&i| rule(lines[i], RULE))?;
    let end = last_filled(lines, below)?;
    let blank = (0..end).rev().find(|&i| lines[i].is_empty())?;
    let options = options(&lines[blank + 1..=end], 0, true, CURSOR)?;
    let (free, offered) = options.split_last()?;
    if free.label != FREE_TEXT || offered.is_empty() || options.iter().any(|o| o.label.contains(CHECKBOX)) {
        return None;
    }
    let question_end = last_filled(lines, blank)?;
    let gap = (0..question_end).rev().find(|&i| lines[i].is_empty())?;
    if question_end - gap > MAX_QUESTION_LINES {
        return None;
    }
    let header = lines.get(gap.checked_sub(1)?)?.strip_prefix(HEADER)?;
    if header.contains('☐') || !rule(lines.get(gap.checked_sub(2)?)?, RULE) {
        return None;
    }
    let title = lines[gap + 1..=question_end].join(" ");
    let choices = offered.iter().map(|o| choice(&o.label, o.number)).collect::<Option<Vec<_>>>()?;
    let fingerprint = lines[gap - 2..=footer].join("\n");
    Some(Ask { question: true, title, detail: header.trim().to_string(), choices, fingerprint })
}

fn last_line(lines: &[&str]) -> Option<usize> {
    lines.iter().rposition(|line| !line.is_empty())
}

fn last_filled(lines: &[&str], before: usize) -> Option<usize> {
    (0..before).rev().find(|&i| !lines[i].is_empty())
}

fn rule(line: &str, c: char) -> bool {
    let line = line.trim();
    line.chars().count() >= RULE_MIN && line.chars().all(|x| x == c)
}

#[cfg(test)]
mod tests;
