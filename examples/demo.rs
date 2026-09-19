//! cargo run --release --example demo [path/to/file.probe]

use infoprober::{parse, Value};
use std::{env, fs, hint::black_box, time::Instant};

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| "tests/data/info.probe".into());
    let src = fs::read_to_string(&path).expect("could not read file");

    // ---- read
    let mut doc = parse(&src).unwrap_or_else(|e| panic!("{path}: {e}"));
    if let Some(Value::Map(widgets)) = doc.get("set", "widgets") {
        for w in widgets {
            println!("widget {:<8} -> {:?}", w.key, w.value.as_bool());
        }
    }

    // ---- timing (parse and render only; no I/O)
    const N: u32 = 200_000;

    let t = Instant::now();
    for _ in 0..N {
        black_box(parse(black_box(&src)).unwrap());
    }
    report("parse ", t.elapsed().as_secs_f64(), N, src.len());

    let mut buf = String::with_capacity(doc.estimated_len());
    let t = Instant::now();
    for _ in 0..N {
        buf.clear();
        doc.render_into(&mut buf);
        black_box(&buf);
    }
    report("render", t.elapsed().as_secs_f64(), N, buf.len());

    // ---- edit + write
    doc.section_or_insert("set").set("dnd", "false");
    doc.validate().expect("document is not writable");
    print!("\n{}", doc.render());
}

fn report(what: &str, secs: f64, n: u32, bytes: usize) {
    let per = secs / n as f64;
    println!(
        "{what}: {:>7.0} ns/iter  {:>7.0} MB/s",
        per * 1e9,
        bytes as f64 / per / 1e6
    );
}
