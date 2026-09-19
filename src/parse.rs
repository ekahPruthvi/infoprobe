use std::borrow::Cow;
use std::iter::Enumerate;
use std::str::Split;

use crate::error::{ParseError, ParseErrorKind};
use crate::model::{Document, Entry, Section, Value};

const MAX_DEPTH: usize = 64;

type Lines<'a> = Enumerate<Split<'a, char>>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Block {
    Section,
    Map,
}

pub fn parse(input: &str) -> Result<Document<'_>, ParseError> {
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    let mut doc = Document::default();
    doc.sections.reserve(4);
    let mut lines: Lines<'_> = input.split('\n').enumerate();

    while let Some((i, raw)) = lines.next() {
        let line = raw.trim_ascii();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let lineno = i + 1;

        if let Some(name) = line.strip_prefix(':') {
            let name = name.trim_ascii_start();
            if name.is_empty() || name == "end" {
                return Err(ParseError::new(lineno, ParseErrorKind::UnexpectedLine));
            }
            let entries = parse_entries(&mut lines, lineno, 0, Block::Section)?;
            doc.sections.push(Section { name: Cow::Borrowed(name), entries });
        } else if let Some((k, v)) = line.split_once('=') {
            doc.meta.push((
                Cow::Borrowed(k.trim_ascii_end()),
                Cow::Borrowed(v.trim_ascii_start()),
            ));
        } else {
            return Err(ParseError::new(lineno, ParseErrorKind::UnexpectedLine));
        }
    }
    Ok(doc)
}

fn parse_entries<'a>(
    lines: &mut Lines<'a>,
    open_line: usize,
    depth: usize,
    block: Block,
) -> Result<Vec<Entry<'a>>, ParseError> {
    if depth > MAX_DEPTH {
        return Err(ParseError::new(open_line, ParseErrorKind::TooDeep));
    }
    let unterminated = match block {
        Block::Section => ParseErrorKind::UnterminatedSection,
        Block::Map => ParseErrorKind::UnterminatedMap,
    };

    let mut entries = Vec::with_capacity(8);
    while let Some((i, raw)) = lines.next() {
        let line = raw.trim_ascii();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let lineno = i + 1;

        // Keys should never start wit ':' only values start with the identifier.
        if line.as_bytes()[0] == b':' {
            if block == Block::Section && line == ":end" {
                return Ok(entries);
            }
            return Err(ParseError::new(open_line, unterminated));
        }

        if line == "}" {
            if block == Block::Map {
                return Ok(entries);
            }
            return Err(ParseError::new(lineno, ParseErrorKind::UnexpectedCloseBrace));
        }

        let (key, val) = split_entry(line, lineno)?;
        let value = if val == "{" {
            Value::Map(parse_entries(lines, lineno, depth + 1, Block::Map)?)
        } else {
            Value::Str(Cow::Borrowed(val))
        };
        entries.push(Entry { key: Cow::Borrowed(key), value });
    }
    Err(ParseError::new(open_line, unterminated))
}

#[inline]
fn split_entry(line: &str, lineno: usize) -> Result<(&str, &str), ParseError> {
    match line.split_once(':') {
        Some((k, v)) => {
            let k = k.trim_ascii_end();
            if k.is_empty() {
                Err(ParseError::new(lineno, ParseErrorKind::EmptyKey))
            } else {
                Ok((k, v.trim_ascii_start()))
            }
        }
        None => Err(ParseError::new(lineno, ParseErrorKind::MissingColon)),
    }
}
