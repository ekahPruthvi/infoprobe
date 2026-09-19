use infoprober::{parse, ParseErrorKind};

const SAMPLE: &str = include_str!("data/info.probe");

#[test]
fn parses_sample() {
    let doc = parse(SAMPLE).unwrap();
    assert_eq!(doc.sections.len(), 2);

    let ver = doc.section("ver").unwrap();
    assert_eq!(ver.entries.len(), 7);
    assert_eq!(ver.get_str("cynageOS"), Some("5.1.1"));
    assert_eq!(ver.get_str("capsuleshell"), Some("1.4.74"));

    let set = doc.section("set").unwrap();
    assert_eq!(set.get_str("startup"), Some("default.mp3"));
    assert_eq!(set.get("dnd").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(set.get_str("shellout"), Some("DP-2"));
    assert_eq!(
        set.get_str("sticker"),
        Some("/var/lib/cynager/icons/skicker/intro.png")
    );

    let widgets = set.get("widgets").and_then(|v| v.as_map()).unwrap();
    assert_eq!(widgets.len(), 5);
    assert_eq!(widgets[0].key, "cal");

    let walls = set.get("walls").unwrap();
    assert_eq!(
        walls.get(".config/walls/cuttingmat.png").and_then(|v| v.as_str()),
        Some("DP-2")
    );
}

#[test]
fn roundtrip_is_structurally_identical() {
    let doc = parse(SAMPLE).unwrap();
    let text = doc.render();
    let doc2 = parse(&text).unwrap();
    assert_eq!(doc, doc2);
    // rendering is stable
    assert_eq!(text, doc2.render());
}

#[test]
fn edit_and_write() {
    let mut doc = parse(SAMPLE).unwrap();
    doc.section_or_insert("set").set("dnd", "false");
    doc.section_or_insert("extra").set("k", String::from("v"));
    assert!(doc.validate().is_ok());

    let text = doc.render();
    let re = parse(&text).unwrap();
    assert_eq!(re.get("set", "dnd").and_then(|v| v.as_str()), Some("false"));
    assert_eq!(re.get("extra", "k").and_then(|v| v.as_str()), Some("v"));
    // an existing key keeps its position when overwritten
    let set = re.section("set").unwrap();
    assert_eq!(set.entries[2].key, "dnd");
}

#[test]
fn validate_rejects_unwritable_data() {
    let mut doc = parse(SAMPLE).unwrap();
    doc.section_or_insert("set").set("bad:key", "x");
    assert!(doc.validate().is_err());
}

#[test]
fn header_lines_and_crlf() {
    let doc = parse("name=demo\r\n\r\n:ver\r\n  a :1\r\n:end\r\n").unwrap();
    assert_eq!(doc.meta[0].0, "name");
    assert_eq!(doc.meta[0].1, "demo");
    assert_eq!(doc.get("ver", "a").and_then(|v| v.as_str()), Some("1"));
    assert_eq!(parse(&doc.render()).unwrap(), doc);
}

#[test]
fn values_may_contain_colons() {
    let doc = parse(":set\n  t :12:30:00\n:end\n").unwrap();
    assert_eq!(doc.get("set", "t").and_then(|v| v.as_str()), Some("12:30:00"));
}

#[test]
fn errors_carry_line_numbers() {
    let e = parse(":ver\n a :1\n").unwrap_err();
    assert_eq!((e.line, e.kind), (1, ParseErrorKind::UnterminatedSection));

    let e = parse(":ver\nfoo\n:end\n").unwrap_err();
    assert_eq!((e.line, e.kind), (2, ParseErrorKind::MissingColon));

    let e = parse(":set\n w :{\n a :1\n:end\n").unwrap_err();
    assert_eq!((e.line, e.kind), (2, ParseErrorKind::UnterminatedMap));

    let e = parse(":a\n}\n:end\n").unwrap_err();
    assert_eq!((e.line, e.kind), (2, ParseErrorKind::UnexpectedCloseBrace));

    let e = parse("stray\n").unwrap_err();
    assert_eq!((e.line, e.kind), (1, ParseErrorKind::UnexpectedLine));
}

#[test]
fn into_owned_outlives_input() {
    let owned = {
        let text = String::from(SAMPLE);
        let doc = parse(&text).unwrap().into_owned();
        doc
    };
    assert_eq!(owned.get("ver", "alt").and_then(|v| v.as_str()), Some("1.2.0"));
}

#[test]
fn comment_lines_are_ignored() {
    let src = "// top\n:set\n  // note\n  a :1\n  w :{\n    // inner\n    b :2\n  }\n  url :http://x.y\n:end\n// bottom\n";
    let doc = parse(src).unwrap();

    let set = doc.section("set").unwrap();
    assert_eq!(set.entries.len(), 3);
    assert_eq!(set.get_str("a"), Some("1"));
    assert_eq!(set.get("w").unwrap().as_map().unwrap().len(), 1);
    // `//` only starts a comment at the beginning of a line
    assert_eq!(set.get_str("url"), Some("http://x.y"));

    // comments are dropped when writing
    let text = doc.render();
    assert!(!text.contains("note") && !text.contains("inner"));
}

#[test]
fn validate_rejects_keys_that_look_like_comments() {
    let mut doc = parse(SAMPLE).unwrap();
    doc.section_or_insert("set").set("//oops", "x");
    assert!(doc.validate().is_err());
}
