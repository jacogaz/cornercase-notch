use crate::ask::{Ask, Choice};

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
const ALLOW: &str = "allow";
const DENY: &str = "deny";
const MAX_QUESTION_LINES: usize = 4;
const OPTION_TEXT: usize = 5;

pub(crate) fn ask_screen(screen: &str) -> Option<Ask> {
    let lines: Vec<&str> = screen.lines().map(str::trim_end).collect();
    permission(&lines).or_else(|| question(&lines))
}

struct Numbered {
    number: u32,
    label: String,
}

fn permission(lines: &[&str]) -> Option<Ask> {
    let footer = lines.iter().rposition(|line| line.starts_with(PERMISSION_FOOTER))?;
    let end = last_filled(lines, footer)?;
    let start = (0..=end).rev().find(|&i| lines[i].starts_with(PROCEED) && lines[i].ends_with('?'))?;
    let options = options(&lines[start + 1..=end], 1, false)?;
    let first = options.first().filter(|o| o.label == YES)?;
    let no = options.iter().find(|o| o.label == NO || o.label.starts_with("No,"))?;
    let top = (0..start).rev().find(|&i| rule(lines[i], RULE))?;
    let title = lines.get(top + 1).map(|line| line.trim()).filter(|t| !t.is_empty() && top + 1 < start)?;
    let dashes: Vec<usize> = (top + 1..start).filter(|&i| rule(lines[i], DASH)).collect();
    let detail = match dashes.as_slice() {
        [open, close, ..] => lines[open + 1..*close].iter().map(|l| l.trim()).collect::<Vec<_>>().join(" "),
        _ => String::new(),
    };
    let choices = vec![choice(ALLOW, first.number)?, choice(DENY, no.number)?];
    Some(Ask { question: false, title: title.to_string(), detail, choices })
}

fn question(lines: &[&str]) -> Option<Ask> {
    let footer = lines.iter().rposition(|line| *line == QUESTION_FOOTER)?;
    let below = (0..footer).rev().find(|&i| rule(lines[i], RULE))?;
    let end = last_filled(lines, below)?;
    let blank = (0..end).rev().find(|&i| lines[i].is_empty())?;
    let options = options(&lines[blank + 1..=end], 0, true)?;
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
    Some(Ask { question: true, title, detail: header.trim().to_string(), choices })
}

fn last_filled(lines: &[&str], before: usize) -> Option<usize> {
    (0..before).rev().find(|&i| !lines[i].is_empty())
}

fn options(lines: &[&str], indent: usize, described: bool) -> Option<Vec<Numbered>> {
    let mut found: Vec<Numbered> = Vec::new();
    let mut cursors = 0;
    let below = " ".repeat(indent + OPTION_TEXT);
    for line in lines {
        if let Some((cursor, number, label)) = line.get(indent..).and_then(option) {
            cursors += usize::from(cursor);
            if number != u32::try_from(found.len()).ok()? + 1 {
                return None;
            }
            found.push(Numbered { number, label: label.to_string() });
        } else if line.starts_with(&below) && described {
            found.last()?;
        } else if line.starts_with(&below) {
            let last = found.last_mut()?;
            last.label.push(' ');
            last.label.push_str(line.trim());
        } else {
            return None;
        }
    }
    (cursors <= 1 && !found.is_empty()).then_some(found)
}

fn option(line: &str) -> Option<(bool, u32, &str)> {
    let mut chars = line.chars();
    let cursor = match chars.next()? {
        CURSOR => true,
        ' ' => false,
        _ => return None,
    };
    let rest = chars.as_str().strip_prefix(' ')?;
    let (number, label) = rest.split_once(". ")?;
    let number = number.parse().ok().filter(|n| (1..=9).contains(n) && number.len() == 1)?;
    (!label.trim().is_empty()).then_some((cursor, number, label.trim()))
}

fn choice(label: &str, number: u32) -> Option<Choice> {
    Some(Choice { label: label.to_string(), key: char::from_digit(number, 10)? })
}

fn rule(line: &str, c: char) -> bool {
    let line = line.trim();
    line.chars().count() >= RULE_MIN && line.chars().all(|x| x == c)
}

#[cfg(test)]
mod tests;
