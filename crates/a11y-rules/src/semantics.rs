//! Tier-2-Regeln: brauchen einen echten Accessible Name.
//!
//! Warum das nicht strukturell geht: Der Accessible Name eines Links oder
//! Buttons entsteht nach einem mehrstufigen Verfahren — `aria-labelledby` löst
//! Verweisketten auf, versteckte Teilbäume zählen unter bestimmten Bedingungen
//! doch mit, `alt` eines eingebetteten Bildes fließt ein. Eine Näherung aus
//! Teilbaumtext plus `aria-label` liegt in genau den Fällen falsch, in denen
//! es darauf ankommt.
//!
//! Hosts ohne [`Semantics`] melden diese Regeln als `UNTESTED` — siehe
//! [`crate::run`].
//!
//! [`Semantics`]: a11y_dom::Semantics

use a11y_dom::{elements, Node, NodeId, Semantics, Tier};
use a11y_report::{Finding, Location, Severity};

use crate::registry::{Meta, SemanticsRule};

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

fn named<'n, D: Semantics>(doc: &'n D, n: D::N<'n>) -> bool {
    doc.accessible_name(n).is_some_and(|s| !s.trim().is_empty())
}

fn link_names<D: Semantics>(doc: &D, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if !n.is_element("a") || !n.has_attr("href") || doc.is_ignored(n) {
            continue;
        }
        if !named(doc, n) {
            out.push(
                Finding::fail(
                    "links/name-missing",
                    "Der Link hat keinen zugänglichen Namen.",
                )
                .with_severity(Severity::Critical)
                .with_wcag(["2.4.4", "4.1.2"])
                .at(at(n.id())),
            );
        }
    }
}

fn button_names<D: Semantics>(doc: &D, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let ist_button = n.is_element("button") || doc.role(n).as_deref() == Some("button");
        if !ist_button || doc.is_ignored(n) {
            continue;
        }
        if !named(doc, n) {
            out.push(
                Finding::fail(
                    "buttons/name-missing",
                    "Der Button hat keinen zugänglichen Namen.",
                )
                .with_severity(Severity::Critical)
                .with_wcag(["4.1.2"])
                .at(at(n.id())),
            );
        }
    }
}

fn svg_names<D: Semantics>(doc: &D, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if !n.is_element("svg") || doc.is_ignored(n) {
            continue;
        }
        // Rein dekoratives SVG ist korrekt ausgezeichnet und gemeint.
        if matches!(n.attr("role"), Some("presentation") | Some("none"))
            || n.attr("aria-hidden") == Some("true")
        {
            continue;
        }
        if !named(doc, n) {
            out.push(
                Finding::fail(
                    "svg/name-missing",
                    "Das SVG hat keinen zugänglichen Namen und ist nicht als dekorativ ausgezeichnet.",
                )
                .with_severity(Severity::High)
                .with_wcag(["1.1.1"])
                .at(at(n.id())),
            );
        }
    }
}

/// Gleicher Linktext, unterschiedliches Ziel — für Screenreader-Nutzer, die
/// sich eine Linkliste ausgeben lassen, nicht unterscheidbar.
fn ambiguous_link_names<D: Semantics>(doc: &D, out: &mut Vec<Finding>) {
    use std::collections::HashMap;
    let mut nach_name: HashMap<String, Vec<(NodeId, String)>> = HashMap::new();

    for n in elements(doc) {
        if !n.is_element("a") || doc.is_ignored(n) {
            continue;
        }
        let (Some(name), Some(href)) = (doc.accessible_name(n), n.attr("href")) else {
            continue;
        };
        let key = name.trim().to_lowercase();
        if key.is_empty() {
            continue;
        }
        nach_name
            .entry(key)
            .or_default()
            .push((n.id(), href.to_string()));
    }

    for (name, treffer) in nach_name {
        if treffer.len() < 2 {
            continue;
        }
        let ziele: std::collections::HashSet<&str> =
            treffer.iter().map(|(_, h)| h.as_str()).collect();
        if ziele.len() < 2 {
            continue; // gleicher Text, gleiches Ziel: unproblematisch
        }
        for (id, _) in &treffer {
            out.push(
                Finding::review(
                    "links/ambiguous-name",
                    format!("Mehrere Links heißen \"{name}\", zeigen aber auf verschiedene Ziele."),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["2.4.4"])
                .at(at(*id)),
            );
        }
    }
}

/// Alle Tier-2-Regeln.
pub fn rules<D: Semantics>() -> Vec<SemanticsRule<D>> {
    macro_rules! rule {
        ($id:literal, $wcag:expr, $sev:expr, $help:literal, $f:path) => {
            SemanticsRule {
                meta: Meta {
                    id: $id,
                    tier: Tier::Semantics,
                    wcag: $wcag,
                    severity: $sev,
                    help: $help,
                },
                run: $f,
            }
        };
    }

    vec![
        rule!(
            "links/name",
            &["2.4.4", "4.1.2"],
            Severity::Critical,
            "Jeder Link braucht einen Namen, der sein Ziel beschreibt.",
            link_names
        ),
        rule!(
            "buttons/name",
            &["4.1.2"],
            Severity::Critical,
            "Jeder Button braucht einen Namen, der seine Wirkung beschreibt.",
            button_names
        ),
        rule!(
            "svg/name",
            &["1.1.1"],
            Severity::High,
            "Informative SVGs brauchen einen Namen, dekorative role=\"presentation\".",
            svg_names
        ),
        rule!(
            "links/ambiguous-name",
            &["2.4.4"],
            Severity::Medium,
            "Gleich benannte Links sollten auf dasselbe Ziel zeigen.",
            ambiguous_link_names
        ),
    ]
}
