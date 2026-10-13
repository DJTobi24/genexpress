//! Translations of the user interface (layer L0).
//!
//! English is the source language: the UI is written with English strings, and each is looked up
//! in the current language's catalogue when it is shown: `tr("Download")`, or with values filled
//! in, `tr!("Downloaded {path}", path = p.display())` and, with a count that picks the plural form,
//! `tr_n!("{n} page", "{n} pages", n)`. A string missing from a catalogue shows in English; nothing
//! here can fail.
//!
//! Catalogues are gettext PO files (`po/<code>.po`, editable with any PO editor) compiled into the
//! program and read on first use. The current language is kept per thread ([`set_language`]):
//! the UI thread sets it from the `ui.setLanguage` preference every frame, and tests running in
//! parallel each keep their own.
#![forbid(unsafe_code)]

mod po;

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::OnceLock;

pub use po::{Catalog, Entry, parse};

/// A language the user interface is available in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Lang {
    /// English, the source language.
    #[default]
    En,
    De,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::En, Lang::De];

    /// The ISO 639-1 code (`"de"`).
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::De => "de",
        }
    }

    /// The language's name in that language (`"Deutsch"`), as language pickers show it.
    pub fn native_name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::De => "Deutsch",
        }
    }

    /// The language of a code or locale name: `de`, `de-AT`, `de_DE.UTF-8`, `German` (any case).
    pub fn parse(s: &str) -> Option<Lang> {
        let s = s.trim().to_ascii_lowercase();
        let primary = s.split(['-', '_', '.', '@']).next().unwrap_or("");
        match primary {
            "en" | "english" => Some(Lang::En),
            "de" | "german" | "deutsch" => Some(Lang::De),
            _ => None,
        }
    }

    /// Which plural form `n` takes (an index into a PO entry's `msgstr[i]`). English and German
    /// both have two: one (n = 1) and other.
    pub fn plural_form(self, n: u64) -> usize {
        match self {
            Lang::En | Lang::De => usize::from(n != 1),
        }
    }

    /// The language's catalogue (none for English, the source language).
    pub fn catalog(self) -> Option<&'static Catalog> {
        match self {
            Lang::En => None,
            Lang::De => {
                static DE: OnceLock<Catalog> = OnceLock::new();
                // A damaged catalogue (a test checks it is not) leaves the UI in English.
                Some(DE.get_or_init(|| parse(include_str!("../po/de.po")).unwrap_or_default()))
            }
        }
    }

    /// The computer's (or the browser's) preferred language among those available: the first of
    /// the user's languages that GenHa has, else English. Asked once per run.
    pub fn system() -> Lang {
        static SYSTEM: OnceLock<Lang> = OnceLock::new();
        *SYSTEM.get_or_init(|| from_locales(sys_locale::get_locales()))
    }
}

/// The first language of `locales` (most preferred first) that is available, else English.
pub fn from_locales(locales: impl IntoIterator<Item = String>) -> Lang {
    locales.into_iter().take(32).find_map(|l| Lang::parse(&l)).unwrap_or(Lang::En)
}

/// The language setting: follow the system, or one language.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Setting {
    #[default]
    System,
    Fixed(Lang),
}

impl Setting {
    /// `"system"`, or a language as [`Lang::parse`] reads it.
    pub fn parse(s: &str) -> Option<Setting> {
        let t = s.trim();
        if t.eq_ignore_ascii_case("system") || t.eq_ignore_ascii_case("auto") {
            return Some(Setting::System);
        }
        Lang::parse(t).map(Setting::Fixed)
    }

    /// How it is stored and reported: `"system"`, `"en"`, `"de"`.
    pub fn id(self) -> &'static str {
        match self {
            Setting::System => "system",
            Setting::Fixed(l) => l.code(),
        }
    }

    /// The language it means on this computer.
    pub fn resolve(self) -> Lang {
        match self {
            Setting::System => Lang::system(),
            Setting::Fixed(l) => l,
        }
    }
}

thread_local! {
    static CURRENT: Cell<Lang> = const { Cell::new(Lang::En) };
}

/// Sets the language strings are translated into on this thread.
pub fn set_language(lang: Lang) {
    CURRENT.with(|c| c.set(lang));
}

/// The language strings are translated into on this thread (English until set).
pub fn language() -> Lang {
    CURRENT.with(Cell::get)
}

/// `key` (an English UI string) in the current language; `key` itself when it has no translation.
pub fn tr(key: &str) -> &str {
    tr_in(language(), key)
}

/// `key` in `lang`; `key` itself when it has no translation.
pub fn tr_in(lang: Lang, key: &str) -> &str {
    lang.catalog().and_then(|c| c.get(key)).unwrap_or(key)
}

/// `key` in `context` (gettext's `msgctxt`), for an English word that needs another translation
/// in one place (the picture filter "Fade" is not the animation "Fade"); else as [`tr`].
pub fn trc<'a>(context: &str, key: &'a str) -> &'a str {
    let lang = language();
    lang.catalog().and_then(|c| c.get_in(context, key)).unwrap_or_else(|| tr_in(lang, key))
}

/// The form of a counted string for `n` in the current language (placeholders not filled in; see
/// [`tr_n!`]). `one` is the English singular (the catalogue's key) and `other` the English plural.
pub fn tr_n<'a>(one: &'a str, other: &'a str, n: u64) -> &'a str {
    tr_n_in(language(), one, other, n)
}

/// [`tr_n`] in `lang`.
pub fn tr_n_in<'a>(lang: Lang, one: &'a str, other: &'a str, n: u64) -> &'a str {
    if let Some(s) = lang.catalog().and_then(|c| c.get_plural(one, lang.plural_form(n))) {
        return s;
    }
    if n == 1 { one } else { other }
}

/// Marks an English UI string kept in a table or constant and translated where it is shown
/// (`tr(label)`), as gettext's `N_`: it returns `s` unchanged, and the check for missing
/// translations finds the strings by it.
#[allow(non_snake_case)]
pub const fn N_(s: &str) -> &str {
    s
}

/// `template` with each `{name}` replaced by its value; unknown placeholders stay as they are.
pub fn fill(template: &str, args: &[(&str, &dyn std::fmt::Display)]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        let (before, after) = rest.split_at(open);
        out.push_str(before);
        let value = after.find('}').and_then(|close| {
            let name = after.get(1..close)?;
            let (_, v) = args.iter().find(|(n, _)| *n == name)?;
            Some((v, close))
        });
        match value {
            Some((v, close)) => {
                out.push_str(&v.to_string());
                rest = after.get(close + 1..).unwrap_or("");
            }
            None => {
                out.push('{');
                rest = after.get(1..).unwrap_or("");
            }
        }
    }
    out.push_str(rest);
    out
}

/// A translated string with values filled in: `tr!("Saved {path}", path = p.display())`.
#[macro_export]
macro_rules! tr {
    ($key:literal $(,)?) => {
        $crate::tr($key).to_string()
    };
    ($key:literal, $($name:ident = $value:expr),+ $(,)?) => {
        $crate::fill($crate::tr($key), &[$((stringify!($name), &$value as &dyn ::std::fmt::Display)),+])
    };
}

/// A counted, translated string: the plural form for `n`, `{n}` and any other values filled in:
/// `tr_n!("{n} page", "{n} pages", pages)`.
#[macro_export]
macro_rules! tr_n {
    ($one:literal, $other:literal, $n:expr $(, $name:ident = $value:expr)* $(,)?) => {{
        let n = $n;
        $crate::fill($crate::tr_n($one, $other, n as u64), &[("n", &n as &dyn ::std::fmt::Display) $(, (stringify!($name), &$value as &dyn ::std::fmt::Display))*])
    }};
}

/// Every translation of `lang` by English key, for checks (plural entries by their singular).
pub fn keys(lang: Lang) -> HashMap<&'static str, &'static Entry> {
    lang.catalog().map(|c| c.entries().map(|e| (e.id.as_str(), e)).collect()).unwrap_or_default()
}

#[cfg(test)]
mod tests;
