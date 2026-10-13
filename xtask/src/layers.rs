//! Dependency layering rules (docs/architecture.md › Layers).

use serde_json::Value;

/// Where a workspace crate sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Layer(u8),
    /// Binaries and build tooling: exempt from the rules.
    Exempt,
}

/// Package names without the `genha-` prefix.
pub const TABLE: &[(&str, Class)] = &[
    ("geom", Class::Layer(0)),
    ("color", Class::Layer(0)),
    ("doc", Class::Layer(1)),
    ("text", Class::Layer(2)),
    ("effects", Class::Layer(2)),
    ("layout", Class::Layer(2)),
    ("templates", Class::Layer(2)),
    ("motion", Class::Layer(2)),
    ("render", Class::Layer(3)),
    ("svg", Class::Layer(3)),
    ("pdf", Class::Layer(3)),
    ("video", Class::Layer(3)),
    ("format", Class::Layer(3)),
    ("tools", Class::Layer(4)),
    ("engine", Class::Layer(5)),
    ("ui-egui", Class::Layer(6)),
    ("mcp", Class::Layer(6)),
    // L7 apps and tooling
    ("genha", Class::Exempt),
    ("cli", Class::Exempt),
    ("web", Class::Exempt),
    ("xtask", Class::Exempt),
];

/// Allowed dependencies *within* a layer (earlier may be used by later).
pub const INTRA_LAYER_ORDER: &[&[&str]] =
    &[&["geom", "color"], &["effects", "templates"], &["layout", "templates"], &["render", "svg"], &["render", "pdf"]];

/// External crates that make up a UI toolkit; only L6 and above may use them.
pub const UI_CRATES: &[&str] = &["egui", "eframe", "winit", "egui_extras", "egui_kittest", "rfd"];
pub const UI_MIN_LAYER: u8 = 6;

pub fn short_name(pkg: &str) -> &str {
    pkg.strip_prefix("genha-").unwrap_or(pkg)
}

pub fn classify(pkg: &str) -> Option<Class> {
    let s = short_name(pkg);
    TABLE.iter().find(|(n, _)| *n == s).map(|(_, c)| *c)
}

fn intra_allowed(from: &str, to: &str) -> bool {
    INTRA_LAYER_ORDER.iter().any(|chain| match (chain.iter().position(|n| *n == from), chain.iter().position(|n| *n == to)) {
        (Some(f), Some(t)) => t < f,
        _ => false,
    })
}

#[derive(Debug, Clone)]
pub struct Crate {
    pub name: String,
    /// (dependency name, is dev-dependency)
    pub deps: Vec<(String, bool)>,
}

pub fn from_metadata(meta: &Value) -> Result<Vec<Crate>, String> {
    let pkgs = meta.get("packages").and_then(Value::as_array).ok_or("cargo metadata: no packages")?;
    Ok(pkgs
        .iter()
        .filter_map(|p| {
            let name = p.get("name")?.as_str()?.to_string();
            let deps = p
                .get("dependencies")?
                .as_array()?
                .iter()
                .filter_map(|d| Some((d.get("name")?.as_str()?.to_string(), d.get("kind").and_then(Value::as_str) == Some("dev"))))
                .collect();
            Some(Crate { name, deps })
        })
        .collect())
}

pub fn check(crates: &[Crate]) -> Vec<String> {
    let mut errors = vec![];
    for c in crates {
        let Some(class) = classify(&c.name) else {
            errors.push(format!("{} is not in the layer table (xtask/src/layers.rs)", c.name));
            continue;
        };
        let Class::Layer(layer) = class else { continue };
        for (dep, dev) in &c.deps {
            if *dev {
                continue;
            }
            if UI_CRATES.contains(&dep.as_str()) && layer < UI_MIN_LAYER {
                errors.push(format!("{} (L{layer}) depends on the UI crate {dep}", c.name));
            }
            if !dep.starts_with("genha-") {
                continue;
            }
            match classify(dep) {
                Some(Class::Layer(dl)) if dl < layer => {}
                Some(Class::Layer(dl)) if dl == layer && intra_allowed(short_name(&c.name), short_name(dep)) => {}
                Some(Class::Layer(dl)) => errors.push(format!("{} (L{layer}) must not depend on {dep} (L{dl})", c.name)),
                _ => errors.push(format!("{} depends on {dep}, which is not a library layer", c.name)),
            }
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(name: &str, deps: &[&str]) -> Crate {
        Crate { name: name.into(), deps: deps.iter().map(|d| (d.to_string(), false)).collect() }
    }

    #[test]
    fn rules() {
        assert!(check(&[c("genha-doc", &["genha-geom", "serde"])]).is_empty());
        assert_eq!(check(&[c("genha-doc", &["genha-engine"])]).len(), 1);
        assert_eq!(check(&[c("genha-engine", &["egui"])]).len(), 1);
        assert!(check(&[c("genha-ui-egui", &["egui", "genha-engine"])]).is_empty());
        assert!(check(&[c("genha-svg", &["genha-render"])]).is_empty());
        assert_eq!(check(&[c("genha-render", &["genha-svg"])]).len(), 1);
        assert!(check(&[c("genha-color", &["genha-geom"])]).is_empty());
        assert_eq!(check(&[c("genha-mystery", &[])]).len(), 1);
        // The web app is exempt like the desktop app: it may use eframe and the UI.
        assert_eq!(classify("genha-web"), Some(Class::Exempt));
        assert!(check(&[c("genha-web", &["genha-ui-egui", "eframe"])]).is_empty());
    }
}
