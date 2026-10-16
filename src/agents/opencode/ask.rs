use crate::ask::{Ask, Choice, DENY};

const BAR: char = '┃';
const GAP: usize = 10;
const SIDEBAR_LINES: usize = 3;
const INDENT: &str = "  ";
const PERMISSION_TITLE: &str = "△ Permission required";
const BUTTONS_WITH_CONFIRMATION: &str = "Allow once   Allow always   Reject";
const BUTTONS_WITHOUT_CONFIRMATION: &str = "Allow once   Always allow   Reject";
const CONFIRM_HINT: &str = "enter confirm";
const QUESTION_FOOTER: &str = "↑↓ select  enter submit  esc dismiss";
const FREE_TEXT: &str = "Type your own answer";
const MULTIPLE: &str = "(select all that apply)";
const STAGES: [&str; 2] = ["△ Always allow", "△ Reject permission"];

/// Sidebar text starts at the same column on a good many lines, after a run of spaces; the least such column
/// is where the main column ends. A screen without one is read whole, and then any text after a run of
/// spaces in a prompt's lines gives no ask.
fn edge(screen: &str) -> usize {
    let mut seen = std::collections::HashMap::new();
    for line in screen.lines() {
        let mut blanks = 0;
        for (column, c) in line.chars().enumerate() {
            if c == ' ' {
                blanks += 1;
                continue;
            }
            if blanks >= GAP {
                *seen.entry(column).or_insert(0) += 1;
            }
            blanks = 0;
        }
    }
    seen.into_iter().filter(|&(_, lines)| lines >= SIDEBAR_LINES).map(|(column, _)| column).min().unwrap_or(usize::MAX)
}

/// One screen line as the main column shows it: a bar, the prompt's text, and what follows a run of ten
/// spaces. What lies right of the main column is the sidebar and is not read at all.
struct Line<'a> {
    bar: bool,
    blank: bool,
    text: &'a str,
    content: &'a str,
    rest: &'a str,
    /// Text reaches the last column of the main column, so the sidebar may have covered more of it.
    touches: bool,
}

impl Line<'_> {
    /// The line is all of one piece of the prompt: nothing after a gap, and nothing the sidebar could cut.
    fn whole(&self) -> bool {
        self.rest.is_empty() && !self.touches
    }
}

fn split(line: &str, edge: usize) -> Line<'_> {
    let at = line.char_indices().nth(edge).map_or(line.len(), |(at, _)| at);
    let main = &line[..at];
    let touches = edge > 0 && edge != usize::MAX && line.chars().nth(edge - 1).is_some_and(|c| c != ' ');
    let (bar, after) = match main.find(BAR) {
        Some(at) => (true, &main[at + BAR.len_utf8()..]),
        None => (false, main),
    };
    let gap = " ".repeat(GAP);
    let lead = after.len() - after.trim_start().len();
    let at = after[lead..].find(&gap).map_or(after.len(), |at| lead + at);
    let (left, rest) = after.split_at(at);
    let left = left.trim_end();
    Line {
        bar,
        blank: main.trim().is_empty() || (bar && after.trim().is_empty()),
        text: left.trim(),
        content: left.strip_prefix(INDENT).unwrap_or(left.trim_start()),
        rest: rest.trim(),
        touches,
    }
}

fn lines(screen: &str) -> Vec<Line<'_>> {
    let edge = edge(screen);
    screen.lines().map(|line| split(line, edge)).collect()
}

/// The lines above a prompt, past blank ones, must not be a prompt's: the first one with text has no bar.
fn bounded_above(lines: &[Line], top: usize) -> bool {
    lines[..top].iter().rev().find(|line| !line.blank).is_none_or(|line| !line.bar)
}

/// Nothing may follow a prompt in the main column: no other prompt, no dialog, no text.
fn nothing_after(lines: &[Line], last: usize) -> bool {
    lines[last + 1..].iter().all(|line| line.blank)
}

pub(crate) fn ask_screen(screen: &str) -> Option<Ask> {
    let lines = lines(screen);
    permission(&lines).or_else(|| question(&lines))
}

/// Whether the screen shows a stage a press should not have led to.
pub(crate) fn stage(screen: &str) -> bool {
    lines(screen).iter().any(|line| line.bar && STAGES.contains(&line.text))
}

/// Enter acts on the highlighted button, which keyboard input or the mouse moves without any change to the
/// text, so a permission is offered no allow: only Esc, which rejects from any highlight.
fn permission(lines: &[Line]) -> Option<Ask> {
    let buttons = lines.iter().rposition(|line| {
        line.bar
            && [BUTTONS_WITH_CONFIRMATION, BUTTONS_WITHOUT_CONFIRMATION].contains(&line.text)
            && line.rest.contains(CONFIRM_HINT)
            && !line.touches
    })?;
    if !nothing_after(lines, buttons) {
        return None;
    }
    let title = lines[..buttons].iter().rposition(|line| line.bar && line.text == PERMISSION_TITLE)?;
    let prompt = &lines[title..=buttons];
    if !prompt.iter().all(|line| line.bar)
        || !prompt[..prompt.len() - 1].iter().all(Line::whole)
        || !bounded_above(lines, title)
    {
        return None;
    }
    let body = &lines[title + 1..buttons];
    let mut shown = body.iter().skip_while(|line| line.text.is_empty()).peekable();
    let heading = shown
        .next_if(|line| line.content.starts_with(INDENT) && !line.content.trim().is_empty())
        .map_or("Permission", |line| line.content.trim().split_once(' ').map_or(line.content.trim(), |(_, name)| name));
    let command: Vec<&Line> = shown.collect();
    let detail = command.iter().map(|line| line.content).collect::<Vec<_>>().join("\n").trim_matches('\n').to_string();
    let fingerprint = prompt.iter().map(|line| line.content).collect::<Vec<_>>().join("\n");
    Some(Ask {
        question: false,
        title: heading.to_string(),
        detail,
        choices: vec![Choice::new(DENY, &["esc"])],
        fingerprint,
    })
}

fn option(content: &str) -> Option<(u32, &str)> {
    let (number, label) = content.split_once(". ")?;
    let number = number.parse().ok().filter(|n| (1..=9).contains(n) && number.len() == 1)?;
    Some((number, label.trim()))
}

fn question(lines: &[Line]) -> Option<Ask> {
    let footer = lines.iter().rposition(|line| line.bar && line.text == QUESTION_FOOTER)?;
    if !nothing_after(lines, footer) {
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
        } else if !(line.blank || (line.content.starts_with("   ") && !rows.is_empty())) {
            return None;
        }
    }
    let asked = (0..first).rev().find(|&i| !lines[i].blank)?;
    let blank = (asked + 1..first).all(|i| lines[i].blank);
    if !lines[asked..=footer].iter().all(Line::whole) {
        return None;
    }
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
