# infoprober

A fast, zero-dependency Rust library for reading and writing `.probe` config files.

- **Reading** is a single zero-copy pass: keys and values are `&str` slices of your input.
- **Writing** pre-sizes one buffer and fills it with `push_str`. No formatting machinery.
- **Order is preserved**, so parse → edit → write gives small diffs.

```toml
# Cargo.toml
[dependencies]
infoprober = 0.1.0
```

```rust
use infoprober::{parse, Document, Entry, Value, ParseErrorKind};
```



## 1. The info.probe file format

```text
// a comment: the whole line is ignored
name=demo                         <- optional header line (top level only)

:ver                              <- ":name" opens a block called "ver"
    cynageOS :5.1.1               <- entry: key, ':', value
    alt :1.2.0
:end                              <- closes the block

:set
    startup :default.mp3
    dnd :true
    widgets :{                    <- a value can be a nested set of entries
        cal   :true
        bat   :true
    }
    walls :{
        .config/walls/a.png :eDP-1
    }
:end
```

Rules:

| Rule | Detail |
|---|---|
| Blocks | `:name` … `:end`. The name `end` is reserved. |
| Entries | `key :value`. Split at the **first** `:`, both sides trimmed, so values may contain colons (`t :12:30:00`). |
| Keys | Any text without `:`. Spaces inside a key are fine (`my key :x`). Paths like `.config/a.png` are fine. |
| Nested maps | `key :{` opens one, a line containing only `}` closes it. They can nest (max depth 64). |
| Comments | A line **starting** with `//` is ignored, anywhere in the file. `url :http://x.y` is *not* a comment. Inline comments are not supported. |
| Whitespace | Indentation and blank lines are ignored. CRLF and a leading BOM are accepted. |
| Header lines | Top-level `key=value` lines (like `name=demo`) are kept in `doc.meta`. |
| Duplicates | Allowed while parsing. Lookups return the **first** match. |

The parser is **strict**: a missing `:end`, a missing `}`, or an entry line with no `:` is an error with a line number rather than being skipped silently.



## 2. Reading

### Load and parse a file

```rust
use infoprober::parse;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let src = fs::read_to_string("info.probe")?;   // keep `src` alive: `doc` borrows from it
    let doc = parse(&src)?;
    println!("{} blocks", doc.sections.len());
    Ok(())
}
```

### Handle errors

```rust
match parse(&src) {
    Ok(doc) => { /* ... */ }
    Err(e) => {
        eprintln!("info.probe: {e}");                 // "line 12: entry is missing ':'"
        if e.kind == ParseErrorKind::UnterminatedSection {
            eprintln!("block opened on line {} never closed", e.line);
        }
    }
}
```

`ParseErrorKind`: `UnexpectedLine`, `MissingColon`, `EmptyKey`, `UnexpectedCloseBrace`, `UnterminatedSection`, `UnterminatedMap`, `TooDeep`. For unterminated blocks, `line` is where the block was **opened**.



## 3. Looking up values

### One value by block and key

```rust
let os: Option<&str> = doc.get("ver", "cynageOS").and_then(|v| v.as_str());
// Some("5.1.1")

let startup = doc.section("set").and_then(|s| s.get_str("startup"));
// Some("default.mp3")
```

### Typed values

```rust
let dnd: Option<bool> = doc.get("set", "dnd").and_then(|v| v.as_bool());   // "true"/"false" only

// any FromStr type (u8, f64, ...). Returns None if missing or not parseable
let volume: Option<u8> = doc.get("set", "volume").and_then(|v| v.parse::<u8>());
```

### Fall back to a default

```rust
let dnd = doc.get("set", "dnd").and_then(|v| v.as_bool()).unwrap_or(false);
```

### Several lookups in one block

Fetch the block once instead of searching for it each time:

```rust
if let Some(set) = doc.section("set") {
    let startup = set.get_str("startup");
    let sticker = set.get_str("sticker");
    let dnd     = set.get("dnd").and_then(|v| v.as_bool());
}
```

### Nested maps

```rust
// Direct lookup inside a nested map
let wall_monitor = doc
    .get("set", "walls")
    .and_then(|w| w.get(".config/walls/cuttingmat.png"))
    .and_then(|v| v.as_str());
// Some("DP-2")

let bat_widget = doc
    .get("set", "widgets")
    .and_then(|w| w.get("bat"))
    .and_then(|v| v.as_bool());
// Some(true)

// Iterate a nested map
if let Some(widgets) = doc.get("set", "widgets").and_then(|v| v.as_map()) {
    for w in widgets {
        println!("{} = {:?}", w.key, w.value.as_bool());
    }
}
```



## 4. Searching

### Iterate everything

```rust
for section in &doc.sections {
    println!(":{}", section.name);
    for e in &section.entries {
        println!("  {}", e.key);
    }
}
```

### Walk the whole tree (nested maps included)

```rust
use infoprober::{Entry, Value};

fn dump(entries: &[Entry<'_>], depth: usize) {
    for e in entries {
        match &e.value {
            Value::Str(s) => println!("{:indent$}{} = {}", "", e.key, s, indent = depth * 2),
            Value::Map(m) => {
                println!("{:indent$}{}:", "", e.key, indent = depth * 2);
                dump(m, depth + 1);
            }
        }
    }
}

for s in &doc.sections {
    println!(":{}", s.name);
    dump(&s.entries, 1);
}
```

### Find all entries with a given value

Which wallpapers are on monitor `DP-2`?

```rust
let walls = doc.get("set", "walls").and_then(|v| v.as_map()).unwrap_or(&[]);

let on_dp2: Vec<&str> = walls
    .iter()
    .filter(|e| e.value.as_str() == Some("DP-2"))
    .map(|e| &*e.key)
    .collect();
// [".config/walls/cuttingmat.png"]
```

### Find a key anywhere in a block (any nesting depth)

```rust
use infoprober::{Entry, Value};

fn find_deep<'d, 'a>(entries: &'d [Entry<'a>], key: &str) -> Option<&'d Value<'a>> {
    for e in entries {
        if e.key == key {
            return Some(&e.value);
        }
        if let Value::Map(m) = &e.value {
            if let Some(v) = find_deep(m, key) {
                return Some(v);
            }
        }
    }
    None
}

let hit = doc.section("set").and_then(|s| find_deep(&s.entries, "bat"));
```

### Find a key across all blocks

```rust
let hits: Vec<(&str, &str)> = doc
    .sections
    .iter()
    .filter_map(|s| s.get_str("startup").map(|v| (&*s.name, v)))
    .collect();
```

### Check what exists

```rust
let has_set_block = doc.section("set").is_some();
let has_dnd       = doc.get("set", "dnd").is_some();
let is_map        = doc.get("set", "widgets").and_then(|v| v.as_map()).is_some();
```

### Header lines

```rust
let name = doc.meta.iter().find(|(k, _)| k == "name").map(|(_, v)| &**v);
```



## 5. Writing

### Edit an existing file and save it

```rust
use infoprober::parse;
use std::fs;

let src = fs::read_to_string("info.probe")?;
let mut doc = parse(&src)?;

// Overwrite a value (keeps its position) or append it if it doesn't exist
doc.section_or_insert("set").set("dnd", "false");

// Remove an entry
if let Some(set) = doc.section_mut("set") {
    set.remove("sticker");
}

doc.validate()?;                          // optional safety check, see below
fs::write("info.probe", doc.render())?;   // `src` is in memory, so overwriting the file is fine
```

### Add or replace a nested map

```rust
use infoprober::Entry;

doc.section_or_insert("set").set(
    "widgets",
    vec![
        Entry::new("cal", "true"),
        Entry::new("sys", "false"),
    ],
);
```

### Change one value inside a nested map

```rust
use infoprober::Value;

if let Some(Value::Map(w)) = doc.section_mut("set").and_then(|s| s.get_mut("widgets")) {
    for e in w.iter_mut() {
        if e.key == "bat" {
            e.value = "false".into();
        }
    }
}
```

### Build a document from scratch

```rust
use infoprober::{Document, Entry};

let mut doc = Document::new();
doc.meta.push(("name".into(), "demo".into()));

let ver = doc.section_or_insert("ver");
ver.push("app", "1.0.0");
ver.push("build", 42.to_string());          // owned Strings work too

let set = doc.section_or_insert("set");
set.push("dnd", "true");
set.push("monitors", vec![Entry::new("main", "DP-1"), Entry::new("side", "DP-2")]);

print!("{}", doc.render());
```

Output:

```text
name=demo

:ver
    app :1.0.0
    build :42
:end

:set
    dnd :true
    monitors :{
        main :DP-1
        side :DP-2
    }
:end
```

`push` appends without checking for duplicates (fastest). `set` checks for an existing key first.

### Write to a file or any `io::Write`

```rust
doc.write_to(std::fs::File::create("out.probe")?)?;   // one write_all call
```

### Reuse one buffer (zero allocations per write)

```rust
let mut buf = String::new();
for _ in 0..1000 {
    buf.clear();                     // keeps the allocation
    doc.render_into(&mut buf);
    // send or save `buf.as_bytes()` ...
}
```

### `validate()`: catching data that can't be written safely

`render()` never checks its input (that is what keeps it fast). If you insert strings you don't control, call `validate()` once before writing. It rejects anything that would not read back identically:

```rust
doc.section_or_insert("set").set("bad:key", "x");
assert!(doc.validate().is_err());
```

It rejects: empty keys; keys containing `:`, a newline, or starting with `//`; values containing a newline or exactly `{`; keys or values with leading/trailing whitespace; block names that are empty, `end`, or contain whitespace.



## 6. Ownership and lifetimes

`parse` returns a `Document<'a>` that borrows from your input string, which is why it is fast. If you need the document to outlive the string (return it from a function, store it in a struct), convert it:

```rust
use infoprober::Document;

fn load(path: &str) -> Result<Document<'static>, Box<dyn std::error::Error>> {
    let src = std::fs::read_to_string(path)?;
    let doc = infoprober::parse(&src)?.into_owned();   // copies every string once
    Ok(doc)
}
```

Values you insert yourself (`String`s, or `&'static str`) are fine in a borrowed document; the string type is `Cow<str>`.



## 7. Getting the best speed

- Read the file **once** into a `String` and parse from that. Don't read it line by line.
- Stay borrowed. Only call `into_owned()` if you must; it copies everything.
- Fetch a `Section` once and use it for several lookups.
- Lookups are linear scans over entries in file order. That beats hashing for typical config sizes; if you have huge blocks, build your own `HashMap<&str, &Value>` from `section.entries` once.
- When writing repeatedly, use `render_into` with a reused buffer.
- Build with `--release`; the included profile enables LTO and a single codegen unit.
- Measure on your files: `cargo run --release --example demo path/to/file.probe`.



## 8. API reference

**`parse(input: &str) -> Result<Document<'_>, ParseError>`**

**`Document`**: fields `meta: Vec<(Str, Str)>`, `sections: Vec<Section>`

| Method | Purpose |
|---|---|
| `new()` | empty document |
| `section(name)` / `section_mut(name)` | find a block |
| `section_or_insert(name)` | find a block or create it at the end |
| `get(section, key)` | shortcut for a value in a block |
| `render()` / `render_into(&mut String)` / `write_to(w)` | serialize |
| `validate()` | check the document can be written and read back unchanged |
| `estimated_len()` | approximate output size (used to pre-size buffers) |
| `into_owned()` | detach from the input string |

**`Section`**: fields `name`, `entries: Vec<Entry>`

| Method | Purpose |
|---|---|
| `new(name)` | empty block |
| `get(key)` / `get_str(key)` / `get_mut(key)` | look up a value (first match) |
| `set(key, value)` | replace the value if the key exists, else append |
| `push(key, value)` | append without checking |
| `remove(key)` | remove and return the value |
| `into_owned()` | detach from the input string |

**`Entry`**: fields `key`, `value`; `Entry::new(key, value)`

**`Value`**: `Str(..)` or `Map(Vec<Entry>)`

| Method | Purpose |
|---|---|
| `as_str()` | plain text, or `None` for a map |
| `as_map()` | nested entries, or `None` for plain text |
| `as_bool()` | `"true"`/`"false"` |
| `parse::<T>()` | any `FromStr` type |
| `get(key)` | look up a key inside a map value |
| `into_owned()` | detach from the input string |

`Value` converts from `&str`, `String`, `Cow<str>` and `Vec<Entry>`, so `set("k", "v")`, `set("k", string)` and `set("k", vec![...])` all work.



## 9. Limitations

- Comments (`//` lines) are dropped when a document is written back.
- Original alignment and spacing are not preserved; the writer uses `key :value` with 4-space indents.
- Only whole-line comments; `key :value // note` keeps `value // note` as the value.
- Nesting deeper than 64 levels is rejected.