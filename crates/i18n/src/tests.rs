use super::*;

const SAMPLE: &str = r#"
# A header, then entries.
msgid ""
msgstr ""
"Language: de\n"
"Plural-Forms: nplurals=2; plural=(n != 1);\n"

#. A comment for translators
msgid "Download"
msgstr "Herunterladen"

msgid "Say \"hi\"\n"
msgstr "Sag "
"\"hallo\"\n"

msgid "{n} page"
msgid_plural "{n} pages"
msgstr[0] "{n} Seite"
msgstr[1] "{n} Seiten"

#, fuzzy
msgid "Unsure"
msgstr "Unsicher"

msgid "Untranslated"
msgstr ""

msgctxt "align"
msgid "Left"
msgstr "Links"
"#;

#[test]
fn po_entries_read() {
    let c = parse(SAMPLE).unwrap();
    assert_eq!(c.get("Download"), Some("Herunterladen"));
    assert_eq!(c.get("Say \"hi\"\n"), Some("Sag \"hallo\"\n"));
    assert_eq!(c.get_plural("{n} page", 0), Some("{n} Seite"));
    assert_eq!(c.get_plural("{n} page", 1), Some("{n} Seiten"));
    // A counted string is not a plain one, and the other way round.
    assert_eq!(c.get("{n} page"), None);
    assert_eq!(c.get_plural("Download", 0), None);
    // Fuzzy and empty translations are not used.
    assert_eq!(c.get("Unsure"), None);
    assert_eq!(c.get("Untranslated"), None);
    assert_eq!(c.get_in("align", "Left"), Some("Links"));
    assert_eq!(c.get("Left"), None);
    assert_eq!(c.len(), 4);
    assert_eq!(c.entries().count(), 6);
}

#[test]
fn po_errors_name_the_line() {
    assert!(parse("msgid \"a\"\nmsgstr \"b\"\n\nmsgid \"a\"\nmsgstr \"c\"\n").unwrap_err().contains("duplicate"));
    assert!(parse("msgid a\n").unwrap_err().contains("line 1"));
    assert!(parse("msgid \"a\"\nmsgfoo \"b\"\n").unwrap_err().contains("line 2"));
    assert!(parse("\"stray\"\n").is_err());
    assert!(parse("msgid \"a\"\nmsgstr[99] \"b\"\n").is_err());
    // Junk never panics.
    for junk in ["\"", "msgid \"\\", "msgstr[", "msgstr[1", "#,", "msgid \"a\" \"b\"", "\u{0}\u{4}\"\""] {
        let _ = parse(junk);
    }
}

#[test]
fn german_catalogue_is_well_formed() {
    let c = parse(include_str!("../po/de.po")).unwrap();
    assert!(c.len() > 500, "{} entries", c.len());
    for e in c.entries() {
        assert!(!e.fuzzy && e.forms.iter().all(|f| !f.is_empty()), "untranslated: {:?}", e.id);
        // Every placeholder of the English text is in the translation, and nothing else.
        let wanted = placeholders(&e.id);
        for f in &e.forms {
            assert_eq!(placeholders(f), wanted, "placeholders differ: {:?} → {f:?}", e.id);
        }
        if let Some(p) = &e.plural {
            assert_eq!(placeholders(p), wanted, "{:?}", e.id);
            assert_eq!(e.forms.len(), 2, "German has two plural forms: {:?}", e.id);
        } else {
            assert_eq!(e.forms.len(), 1, "{:?}", e.id);
        }
        // Mistakes that are easy to make when writing a catalogue by hand.
        for f in &e.forms {
            assert_eq!((e.id.starts_with(' '), e.id.ends_with(' ')), (f.starts_with(' '), f.ends_with(' ')), "spaces: {:?} → {f:?}", e.id);
            assert_eq!(e.id.ends_with('…'), f.ends_with('…'), "ellipsis: {:?} → {f:?}", e.id);
            assert_eq!(e.id.ends_with(':'), f.ends_with(':'), "colon: {:?} → {f:?}", e.id);
            assert_eq!(e.id.contains('\n'), f.contains('\n'), "line break: {:?} → {f:?}", e.id);
        }
    }
}

fn placeholders(s: &str) -> Vec<String> {
    let mut v: Vec<String> = s.split('{').skip(1).filter_map(|p| p.split_once('}').map(|(n, _)| n.to_string())).collect();
    v.sort();
    v
}

#[test]
fn lookups_fall_back_to_english() {
    set_language(Lang::De);
    assert_eq!(tr("Download"), "Herunterladen");
    assert_eq!(tr("Not a key anyone wrote"), "Not a key anyone wrote");
    assert_eq!(tr_in(Lang::En, "Download"), "Download");
    set_language(Lang::En);
    assert_eq!(tr("Download"), "Download");
}

#[test]
fn plurals_and_values() {
    set_language(Lang::De);
    assert_eq!(tr_n!("{n} page", "{n} pages", 1), "1 Seite");
    assert_eq!(tr_n!("{n} page", "{n} pages", 0), "0 Seiten");
    assert_eq!(tr_n!("{n} page", "{n} pages", 2_usize), "2 Seiten");
    assert_eq!(tr_n!("{n} page", "{n} pages", 5_u64), "5 Seiten");
    // Missing counted strings: the English forms.
    assert_eq!(tr_n!("{n} gizmo", "{n} gizmos", 1), "1 gizmo");
    assert_eq!(tr_n!("{n} gizmo", "{n} gizmos", 3), "3 gizmos");
    assert_eq!(tr!("Saved {path}", path = "/tmp/a.genha"), "/tmp/a.genha gespeichert");
    set_language(Lang::En);
    assert_eq!(tr_n!("{n} page", "{n} pages", 1), "1 page");
    assert_eq!(tr_n!("{n} page", "{n} pages", 7), "7 pages");
    assert_eq!(tr!("Saved {path}", path = "x"), "Saved x");
}

#[test]
fn fill_replaces_named_values_only() {
    assert_eq!(fill("{a} and {b}", &[("a", &1), ("b", &"two")]), "1 and two");
    assert_eq!(fill("{a} {missing} {", &[("a", &1)]), "1 {missing} {");
    assert_eq!(fill("{}{{a}}", &[("a", &"x")]), "{}{x}");
    assert_eq!(fill("ü{a}ß", &[("a", &"ö")]), "üöß");
}

#[test]
fn languages_parse_and_resolve() {
    assert_eq!(Lang::parse("de"), Some(Lang::De));
    assert_eq!(Lang::parse("de_DE.UTF-8"), Some(Lang::De));
    assert_eq!(Lang::parse("DE-at"), Some(Lang::De));
    assert_eq!(Lang::parse("en-US"), Some(Lang::En));
    assert_eq!(Lang::parse("fr-FR"), None);
    assert_eq!(Lang::parse(""), None);
    assert_eq!(from_locales(["fr-FR".to_string(), "de-CH".to_string()]), Lang::De);
    assert_eq!(from_locales(["ja-JP".to_string()]), Lang::En, "unknown languages: English");
    assert_eq!(from_locales(Vec::<String>::new()), Lang::En);
    assert_eq!(Setting::parse("System"), Some(Setting::System));
    assert_eq!(Setting::parse("de"), Some(Setting::Fixed(Lang::De)));
    assert_eq!(Setting::parse("klingon"), None);
    assert_eq!(Setting::Fixed(Lang::De).resolve(), Lang::De);
    assert_eq!(Setting::Fixed(Lang::De).id(), "de");
    assert!(Lang::ALL.contains(&Setting::System.resolve()));
}

#[test]
fn each_thread_keeps_its_language() {
    set_language(Lang::De);
    let other = std::thread::spawn(language).join().unwrap();
    assert_eq!(other, Lang::En);
    assert_eq!(language(), Lang::De);
    set_language(Lang::En);
}
