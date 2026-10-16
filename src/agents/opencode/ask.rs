use crate::ask::{ALLOW, Ask, Choice, DENY};

const BAR: char = '┃';
const GAP: usize = 10;
const INDENT: &str = "  ";
const PERMISSION_TITLE: &str = "△ Permission required";
const BUTTONS_WITH_CONFIRMATION: &str = "Allow once   Allow always   Reject";
const BUTTONS_WITHOUT_CONFIRMATION: &str = "Allow once   Always allow   Reject";
const CONFIRM_HINT: &str = "enter confirm";
const QUESTION_FOOTER: &str = "↑↓ select  enter submit  esc dismiss";
const FREE_TEXT: &str = "Type your own answer";
const MULTIPLE: &str = "(select all that apply)";
const STAGES: [&str; 2] = ["△ Always allow", "△ Reject permission"];

/// One screen line read the way the prompt shows it: a bar, the prompt's text, and, on the same line,
/// the sidebar, which starts where a run of ten spaces does.
struct Line<'a> {
    bar: bool,
    text: &'a str,
    content: &'a str,
    rest: &'a str,
}

impl Line<'_> {
    /// Text of the prompt may have been cut off by the sidebar: there is text, and more after the gap.
    fn cut(&self) -> bool {
        !self.text.is_empty() && !self.rest.is_empty()
    }
}

fn split(line: &str) -> Line<'_> {
    let (bar, after) = match line.find(BAR) {
        Some(at) => (true, &line[at + BAR.len_utf8()..]),
        None => (false, line),
    };
    let gap = " ".repeat(GAP);
    let at = after.find(&gap).unwrap_or(after.len());
    let (left, rest) = after.split_at(at);
    let left = left.trim_end();
    Line { bar, text: left.trim(), content: left.strip_prefix(INDENT).unwrap_or(left.trim_start()), rest: rest.trim() }
}

/// The lines above a prompt, past those with nothing of the prompt's column, must not be a prompt's: the
/// first one with text has no bar.
fn bounded_above(lines: &[Line], top: usize) -> bool {
    lines[..top].iter().rev().find(|line| !line.text.is_empty()).is_none_or(|line| !line.bar)
}

fn only_sidebar_after(lines: &[Line], last: usize) -> bool {
    lines[last + 1..].iter().all(|line| line.text.is_empty())
}

pub(crate) fn ask_screen(screen: &str) -> Option<Ask> {
    let lines: Vec<Line> = screen.lines().map(split).collect();
    permission(&lines).or_else(|| question(&lines))
}

/// Whether the screen shows a stage a press should not have led to.
pub(crate) fn stage(screen: &str) -> bool {
    screen.lines().map(split).any(|line| line.bar && STAGES.contains(&line.text))
}

fn permission(lines: &[Line]) -> Option<Ask> {
    let buttons = lines.iter().rposition(|line| {
        line.bar
            && [BUTTONS_WITH_CONFIRMATION, BUTTONS_WITHOUT_CONFIRMATION].contains(&line.text)
            && line.rest.contains(CONFIRM_HINT)
    })?;
    if !only_sidebar_after(lines, buttons) {
        return None;
    }
    let confirmation_stage = lines[buttons].text == BUTTONS_WITH_CONFIRMATION;
    let title = lines[..buttons].iter().rposition(|line| line.bar && line.text == PERMISSION_TITLE)?;
    if !lines[title..=buttons].iter().all(|line| line.bar) || !bounded_above(lines, title) {
        return None;
    }
    let body = &lines[title + 1..buttons];
    let mut shown = body.iter().skip_while(|line| line.text.is_empty()).peekable();
    let heading = shown
        .next_if(|line| line.content.starts_with(INDENT) && !line.content.trim().is_empty())
        .map_or("Permission", |line| line.content.trim().split_once(' ').map_or(line.content.trim(), |(_, name)| name));
    let command: Vec<&Line> = shown.collect();
    let cut = command.iter().any(|line| line.cut());
    let detail = command.iter().map(|line| line.content).collect::<Vec<_>>().join("\n").trim_matches('\n').to_string();
    let mut choices = Vec::new();
    if confirmation_stage && !cut {
        choices.push(Choice::new(ALLOW, &["enter"]));
    }
    choices.push(Choice::new(DENY, &["esc"]));
    let fingerprint = lines[title..=buttons].iter().map(|line| line.content).collect::<Vec<_>>().join("\n");
    Some(Ask { question: false, title: heading.to_string(), detail, choices, fingerprint })
}

fn option(content: &str) -> Option<(u32, &str)> {
    let (number, label) = content.split_once(". ")?;
    let number = number.parse().ok().filter(|n| (1..=9).contains(n) && number.len() == 1)?;
    Some((number, label.trim()))
}

fn question(lines: &[Line]) -> Option<Ask> {
    let footer = lines.iter().rposition(|line| line.bar && line.text == QUESTION_FOOTER)?;
    if !only_sidebar_after(lines, footer) {
        return None;
    }
    let first = (0..footer).rev().find(|&i| lines[i].bar && option(lines[i].content).is_some_and(|(n, _)| n == 1))?;
    let mut rows: Vec<&str> = Vec::new();
    for line in &lines[first..footer] {
        if !line.bar {
            return None;
        }
        if let Some((number, label)) = option(line.content) {
            if usize::try_from(number).ok()? != rows.len() + 1 {
                return None;
            }
            rows.push(label);
        } else if !(line.text.is_empty() || (line.content.starts_with("   ") && !rows.is_empty())) {
            return None;
        }
    }
    let asked = (0..first).rev().find(|&i| !lines[i].text.is_empty())?;
    let blank = (asked + 1..first).all(|i| lines[i].text.is_empty());
    if !blank || !lines[asked].bar || lines[asked].content.starts_with(' ') || lines[asked].text.ends_with(MULTIPLE) {
        return None;
    }
    if !bounded_above(lines, asked) {
        return None;
    }
    let offered = match rows.split_last() {
        Some((last, rest)) if last.starts_with(FREE_TEXT) => rest,
        _ => &rows[..],
    };
    if offered.is_empty() || offered.iter().any(|label| label.starts_with(FREE_TEXT)) {
        return None;
    }
    let choices = offered
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let digit = char::from_digit(u32::try_from(i + 1).ok()?, 10)?.to_string();
            Some(Choice::new(label, &[&digit]))
        })
        .collect::<Option<Vec<_>>>()?;
    let fingerprint = lines[asked..=footer].iter().map(|line| line.content).collect::<Vec<_>>().join("\n");
    Some(Ask { question: true, title: lines[asked].text.to_string(), detail: String::new(), choices, fingerprint })
}

#[cfg(test)]
mod tests;
