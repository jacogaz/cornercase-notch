use crate::ask::Choice;

const OPTION_TEXT: usize = 5;

pub(super) struct Numbered {
    pub(super) number: u32,
    pub(super) label: String,
}

pub(super) fn options(lines: &[&str], indent: usize, described: bool, cursor: char) -> Option<Vec<Numbered>> {
    let mut found: Vec<Numbered> = Vec::new();
    let mut cursors = 0;
    let below = " ".repeat(indent + OPTION_TEXT);
    for line in lines {
        if let Some((cursor, number, label)) = line.get(indent..).and_then(|line| option(line, cursor)) {
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

fn option(line: &str, cursor_mark: char) -> Option<(bool, u32, &str)> {
    let mut chars = line.chars();
    let cursor = match chars.next()? {
        c if c == cursor_mark => true,
        ' ' => false,
        _ => return None,
    };
    let rest = chars.as_str().strip_prefix(' ')?;
    let (number, label) = rest.split_once(". ")?;
    let number = number.parse().ok().filter(|n| (1..=9).contains(n) && number.len() == 1)?;
    (!label.trim().is_empty()).then_some((cursor, number, label.trim()))
}

pub(super) fn choice(label: &str, number: u32) -> Option<Choice> {
    Some(Choice { label: label.to_string(), keys: vec![char::from_digit(number, 10)?.to_string()] })
}
