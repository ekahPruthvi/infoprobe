use std::io;

use crate::error::InvalidData;
use crate::model::{Document, Entry, Value};

const INDENT: &str = "    ";

impl Document<'_> {
    pub fn render(&self) -> String {
        let mut out = String::with_capacity(self.estimated_len());
        self.render_into(&mut out);
        out
    }

    pub fn render_into(&self, out: &mut String) {
        out.reserve(self.estimated_len());

        for (k, v) in &self.meta {
            out.push_str(k);
            out.push('=');
            out.push_str(v);
            out.push('\n');
        }

        for (i, section) in self.sections.iter().enumerate() {
            if i > 0 || !self.meta.is_empty() {
                out.push('\n');
            }
            out.push(':');
            out.push_str(&section.name);
            out.push('\n');
            write_entries(out, &section.entries, 1);
            out.push_str(":end\n");
        }
    }

    pub fn write_to<W: io::Write>(&self, mut w: W) -> io::Result<()> {
        w.write_all(self.render().as_bytes())
    }

    pub fn estimated_len(&self) -> usize {
        let mut n = 0;
        for (k, v) in &self.meta {
            n += k.len() + v.len() + 2;
        }
        for s in &self.sections {
            n += s.name.len() + 2 + 5 + 1;
            n += entries_len(&s.entries, 1);
        }
        n
    }

    pub fn validate(&self) -> Result<(), InvalidData> {
        for (k, v) in &self.meta {
            let k: &str = k;
            let v: &str = v;
            if k.is_empty()
                || k.starts_with(':')
                || k.starts_with("//")
                || k.contains(['=', '\n', '\r'])
                || k != k.trim_ascii()
                || v.contains(['\n', '\r'])
                || v != v.trim_ascii()
            {
                return Err(InvalidData("bad header line"));
            }
        }
        for s in &self.sections {
            let name: &str = &s.name;
            if name.is_empty() || name == "end" || name.bytes().any(|b| b.is_ascii_whitespace()) {
                return Err(InvalidData("bad section name"));
            }
            validate_entries(&s.entries)?;
        }
        Ok(())
    }
}

fn write_entries(out: &mut String, entries: &[Entry<'_>], depth: usize) {
    for e in entries {
        push_indent(out, depth);
        out.push_str(&e.key);
        out.push_str(" :");
        match &e.value {
            Value::Str(s) => {
                out.push_str(s);
                out.push('\n');
            }
            Value::Map(m) => {
                out.push_str("{\n");
                write_entries(out, m, depth + 1);
                push_indent(out, depth);
                out.push_str("}\n");
            }
        }
    }
}

#[inline]
fn push_indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str(INDENT);
    }
}

fn entries_len(entries: &[Entry<'_>], depth: usize) -> usize {
    let indent = depth * INDENT.len();
    let mut n = 0;
    for e in entries {
        n += indent + e.key.len() + 2;
        match &e.value {
            Value::Str(s) => n += s.len() + 1,
            Value::Map(m) => {
                n += 2 + entries_len(m, depth + 1) + indent + 2;
            }
        }
    }
    n
}

fn validate_entries(entries: &[Entry<'_>]) -> Result<(), InvalidData> {
    for e in entries {
        let k: &str = &e.key;
        if k.is_empty()
            || k.starts_with("//")
            || k.contains([':', '\n', '\r'])
            || k != k.trim_ascii()
        {
            return Err(InvalidData("bad key"));
        }
        match &e.value {
            Value::Str(v) => {
                let v: &str = v;
                if v.contains(['\n', '\r']) || v != v.trim_ascii() || v == "{" {
                    return Err(InvalidData("bad value"));
                }
            }
            Value::Map(m) => validate_entries(m)?,
        }
    }
    Ok(())
}
