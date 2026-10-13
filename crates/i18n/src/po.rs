//! A reader for gettext PO catalogues (the subset translators' tools write): `msgid`,
//! `msgid_plural`, `msgstr`, `msgstr[i]`, `msgctxt`, continued strings, comments and the `fuzzy`
//! flag. Entries without a translation (empty or fuzzy) are left out, so they show in English.

use std::collections::HashMap;

/// One translated message.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Entry {
    /// Disambiguating context (`msgctxt`), if any.
    pub context: Option<String>,
    /// The English source (the singular for counted strings).
    pub id: String,
    /// The English plural, for counted strings.
    pub plural: Option<String>,
    /// The translation; one per plural form for counted strings.
    pub forms: Vec<String>,
    /// Marked as needing review: not used.
    pub fuzzy: bool,
}

impl Entry {
    fn usable(&self) -> bool {
        !self.fuzzy && !self.id.is_empty() && !self.forms.is_empty() && self.forms.iter().all(|f| !f.is_empty())
    }
}

/// A language's translations.
#[derive(Clone, Debug, Default)]
pub struct Catalog {
    entries: Vec<Entry>,
    /// Usable entries by key (`context \u{4} id` with a context).
    index: HashMap<String, usize>,
}

fn key(context: Option<&str>, id: &str) -> String {
    match context {
        Some(c) => format!("{c}\u{4}{id}"),
        None => id.to_string(),
    }
}

impl Catalog {
    /// The translation of a plain (uncounted) string.
    pub fn get(&self, id: &str) -> Option<&str> {
        let e = self.entries.get(*self.index.get(id)?)?;
        if e.plural.is_some() {
            return None;
        }
        e.forms.first().map(String::as_str)
    }

    /// Plural form `form` of a counted string whose English singular is `one`.
    pub fn get_plural(&self, one: &str, form: usize) -> Option<&str> {
        let e = self.entries.get(*self.index.get(one)?)?;
        e.plural.as_ref()?;
        e.forms.get(form).or(e.forms.last()).map(String::as_str)
    }

    /// The translation of `id` in `context`.
    pub fn get_in(&self, context: &str, id: &str) -> Option<&str> {
        let e = self.entries.get(*self.index.get(&key(Some(context), id))?)?;
        e.forms.first().map(String::as_str)
    }

    /// Every entry, usable or not, in file order.
    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter()
    }

    pub fn len(&self) -> usize {
        self.index.len()
    }

    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Field {
    None,
    Context,
    Id,
    Plural,
    Str(usize),
}

/// Most plural forms an entry may have (the largest gettext languages have six).
const MAX_FORMS: usize = 8;

/// Reads a PO catalogue. Errors name the line; duplicate entries are an error too (one would
/// silently win).
pub fn parse(src: &str) -> Result<Catalog, String> {
    let mut cat = Catalog::default();
    let mut cur = Entry::default();
    let mut started = false;
    let mut field = Field::None;
    let mut seen = std::collections::HashSet::new();
    let mut flush = |cur: &mut Entry, started: &mut bool, line: usize| -> Result<(), String> {
        if !*started {
            return Ok(());
        }
        let e = std::mem::take(cur);
        *started = false;
        // The header (msgid "") carries metadata only.
        if e.id.is_empty() {
            return Ok(());
        }
        let k = key(e.context.as_deref(), &e.id);
        if !seen.insert(k.clone()) {
            return Err(format!("line {line}: duplicate entry for {:?}", e.id));
        }
        if e.usable() {
            cat.index.insert(k, cat.entries.len());
        }
        cat.entries.push(e);
        Ok(())
    };
    let mut fuzzy_next = false;
    for (n, raw) in src.lines().enumerate() {
        let line_no = n + 1;
        let line = raw.trim();
        if line.is_empty() {
            flush(&mut cur, &mut started, line_no)?;
            field = Field::None;
            continue;
        }
        if let Some(flags) = line.strip_prefix("#,") {
            fuzzy_next |= flags.split(',').any(|f| f.trim() == "fuzzy");
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let (kw, rest) = match line.find('"') {
            Some(i) => (line.get(..i).unwrap_or("").trim(), line.get(i..).unwrap_or("")),
            None => return Err(format!("line {line_no}: expected a quoted string")),
        };
        let text = unquote(rest).ok_or_else(|| format!("line {line_no}: bad string {rest}"))?;
        let next = match kw {
            "" => field,
            "msgctxt" => Field::Context,
            "msgid" => Field::Id,
            "msgid_plural" => Field::Plural,
            "msgstr" => Field::Str(0),
            k => match k.strip_prefix("msgstr[").and_then(|r| r.strip_suffix(']')).and_then(|i| i.parse::<usize>().ok()) {
                Some(i) if i < MAX_FORMS => Field::Str(i),
                _ => return Err(format!("line {line_no}: unknown keyword {k}")),
            },
        };
        // A new entry begins at msgctxt, or at msgid unless a msgctxt just opened it.
        let begins = match next {
            Field::Context => true,
            Field::Id => kw == "msgid" && field != Field::Context,
            _ => false,
        };
        if begins {
            flush(&mut cur, &mut started, line_no)?;
        }
        if !kw.is_empty() {
            started = true;
            if fuzzy_next {
                cur.fuzzy = true;
                fuzzy_next = false;
            }
        }
        field = next;
        match field {
            Field::None => return Err(format!("line {line_no}: a string outside an entry")),
            Field::Context => cur.context.get_or_insert_with(String::new).push_str(&text),
            Field::Id => cur.id.push_str(&text),
            Field::Plural => cur.plural.get_or_insert_with(String::new).push_str(&text),
            Field::Str(i) => {
                while cur.forms.len() <= i {
                    cur.forms.push(String::new());
                }
                if let Some(f) = cur.forms.get_mut(i) {
                    f.push_str(&text);
                }
            }
        }
    }
    let end = src.lines().count() + 1;
    flush(&mut cur, &mut started, end)?;
    Ok(cat)
}

/// The text of a C-style quoted string (`"a \"b\"\n"`).
fn unquote(s: &str) -> Option<String> {
    let inner = s.strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '"' {
            return None;
        }
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next()? {
            'n' => out.push('\n'),
            't' => out.push('\t'),
            'r' => out.push('\r'),
            other => out.push(other),
        }
    }
    Some(out)
}
