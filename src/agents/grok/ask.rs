use super::super::numbered::choice;
use crate::ask::{ALLOW, Ask, Choice, DENY};

const ALLOW_LABELS: [&str; 2] = ["Yes, proceed", "Yes"];
const DENY_LABEL: &str = "No, reject (type to add feedback)";
const TITLE_START: &str = "Allow ";
const EXPAND_HINT: &str = "Ctrl-F";
const FREE_TEXT: char = 'z';

pub(crate) fn ask_screen(screen: &str) -> Option<Ask> {
    let lines: Vec<&str> = screen.lines().map(str::trim_end).collect();
    let last = lines.iter().rposition(|line| !line.is_empty())?;
    let card: Vec<Option<&str>> = lines[..=last].iter().map(|line| content(line)).collect();
    let end = card.len() - 1;
    card[end]?;
    let first = (0..=end).rev().take_while(|&i| card[i].is_some()).last()?;
    let body: Vec<&str> = card[first..=end].iter().map(|line| line.unwrap_or_default()).collect();
    permission(&body).or_else(|| question(&body))
}

/// A line of the card: an accent bar, two spaces, then the text; or just the bar.
fn content(line: &str) -> Option<&str> {
    let mut chars = line.chars();
    let bar = chars.next().filter(|c| !c.is_alphanumeric() && !c.is_whitespace())?;
    let rest = &line[bar.len_utf8()..];
    if rest.trim().is_empty() { Some("") } else { rest.strip_prefix("  ") }
}

/// `N (●) label` or `N (○) label`, N being a digit or the free-text key.
fn row(line: &str) -> Option<(char, bool, &str)> {
    let mut chars = line.chars();
    let key = chars.next().filter(char::is_ascii_alphanumeric)?;
    let rest = chars.as_str().strip_prefix(" (")?;
    let (mark, label) = rest.split_once(") ")?;
    let cursor = match mark {
        "●" => true,
        "○" => false,
        _ => return None,
    };
    (!label.trim().is_empty()).then_some((key, cursor, label.trim()))
}

type Row = (char, bool, String);

fn rows(body: &[&str]) -> Option<(usize, Vec<Row>)> {
    let last = body.iter().rposition(|line| !line.is_empty())?;
    let start = (0..=last).rev().take_while(|&i| row(body[i]).is_some()).last()?;
    let found = body[start..=last]
        .iter()
        .map(|line| row(line).map(|(k, c, l)| (k, c, l.to_string())))
        .collect::<Option<Vec<_>>>()?;
    Some((start, found))
}

fn permission(body: &[&str]) -> Option<Ask> {
    let (start, found) = rows(body)?;
    if found.iter().any(|(key, _, _)| *key == FREE_TEXT) || found.len() > 9 {
        return None;
    }
    let digits_in_order = found
        .iter()
        .enumerate()
        .all(|(i, (key, _, _))| key.to_digit(10).and_then(|d| usize::try_from(d).ok()) == Some(i + 1));
    if !digits_in_order || found.iter().filter(|(_, cursor, _)| *cursor).count() != 1 {
        return None;
    }
    let title = (0..start).rev().find(|&i| body[i].starts_with(TITLE_START) && body[i].ends_with('?'))?;
    let mut command = Vec::new();
    let mut elided = false;
    for line in &body[title + 1..start] {
        if line.is_empty() || line.starts_with('←') {
            continue;
        }
        if line.starts_with("...") && line.contains(EXPAND_HINT) {
            elided = true;
        } else {
            command.push(*line);
        }
    }
    let number =
        |label: &dyn Fn(&str) -> bool| found.iter().find(|(_, _, l)| label(l)).and_then(|(k, _, _)| k.to_digit(10));
    let allow = number(&|l| ALLOW_LABELS.contains(&l))?;
    let deny = number(&|l| l == DENY_LABEL)?;
    let mut choices = Vec::new();
    if !elided {
        choices.push(choice(ALLOW, allow)?);
    }
    choices.push(choice(DENY, deny)?);
    Some(Ask {
        question: false,
        title: body[title].to_string(),
        detail: command.join("\n"),
        choices,
        fingerprint: body[title..].join("\n"),
    })
}

fn question(body: &[&str]) -> Option<Ask> {
    let (start, found) = rows(body)?;
    let (free, offered) = found.split_last()?;
    if free.0 != FREE_TEXT || offered.is_empty() || offered.len() > 9 {
        return None;
    }
    let in_order = offered
        .iter()
        .enumerate()
        .all(|(i, (key, _, _))| key.to_digit(10).and_then(|d| usize::try_from(d).ok()) == Some(i + 1));
    if !in_order || offered.iter().any(|(key, _, _)| *key == FREE_TEXT) {
        return None;
    }
    let asked: Vec<&str> = body[..start].iter().copied().filter(|line| !line.is_empty()).collect();
    if asked.is_empty() {
        return None;
    }
    let choices =
        offered.iter().map(|(key, _, label)| choice(label, key.to_digit(10)?)).collect::<Option<Vec<Choice>>>()?;
    Some(Ask { question: true, title: asked.join(" "), detail: String::new(), choices, fingerprint: body.join("\n") })
}

#[cfg(test)]
mod tests;
