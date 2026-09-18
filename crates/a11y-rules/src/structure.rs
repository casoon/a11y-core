//! Tier-1-Regeln: entscheidbar allein aus Tags, Attributen, Text und Hierarchie.

use std::collections::{HashMap, HashSet};

use a11y_dom::{closest, elements, subtree_text, Document, Node, NodeId, Tier};
use a11y_report::{Finding, Location, Severity};

use crate::registry::{Meta, StructureRule};

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

/// Alle gültigen ARIA-Rollen aus WAI-ARIA 1.2, ohne die abstrakten.
pub(crate) const VALID_ROLES: &[&str] = &[
    "alert",
    "alertdialog",
    "application",
    "article",
    "banner",
    "blockquote",
    "button",
    "caption",
    "cell",
    "checkbox",
    "code",
    "columnheader",
    "combobox",
    "complementary",
    "contentinfo",
    "definition",
    "deletion",
    "dialog",
    "directory",
    "document",
    "emphasis",
    "feed",
    "figure",
    "form",
    "generic",
    "grid",
    "gridcell",
    "group",
    "heading",
    "img",
    "insertion",
    "link",
    "list",
    "listbox",
    "listitem",
    "log",
    "main",
    "mark",
    "marquee",
    "math",
    "menu",
    "menubar",
    "menuitem",
    "menuitemcheckbox",
    "menuitemradio",
    "meter",
    "navigation",
    "none",
    "note",
    "option",
    "paragraph",
    "presentation",
    "progressbar",
    "radio",
    "radiogroup",
    "region",
    "row",
    "rowgroup",
    "rowheader",
    "scrollbar",
    "search",
    "searchbox",
    "separator",
    "slider",
    "spinbutton",
    "status",
    "strong",
    "subscript",
    "superscript",
    "switch",
    "tab",
    "table",
    "tablist",
    "tabpanel",
    "term",
    "textbox",
    "time",
    "timer",
    "toolbar",
    "tooltip",
    "tree",
    "treegrid",
    "treeitem",
];

/// Abstrakte Rollen. Sie beschreiben die Taxonomie und dürfen nie im Markup stehen.
const ABSTRACT_ROLES: &[&str] = &[
    "command",
    "composite",
    "input",
    "landmark",
    "range",
    "roletype",
    "section",
    "sectionhead",
    "select",
    "structure",
    "widget",
    "window",
];

fn heading_level(tag: &str) -> Option<u8> {
    let b = tag.as_bytes();
    (b.len() == 2 && b[0] == b'h' && b[1].is_ascii_digit() && (b'1'..=b'6').contains(&b[1]))
        .then(|| b[1] - b'0')
}

// --- Dokument -------------------------------------------------------------

fn lang<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    let root = doc.root();
    if !root.is_element("html") {
        return;
    }
    match root.attr("lang") {
        None => out.push(
            Finding::fail(
                "document/lang-missing",
                "Das <html>-Element hat kein lang-Attribut.",
            )
            .with_severity(Severity::High)
            .with_wcag(["3.1.1"])
            .at(at(root.id())),
        ),
        Some(l) if !is_plausible_lang(l) => out.push(
            Finding::fail(
                "document/lang-invalid",
                format!("Der Sprachcode \"{l}\" ist kein gültiges BCP-47-Kürzel."),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["3.1.1"])
            .at(at(root.id())),
        ),
        _ => {}
    }
}

/// Grobprüfung auf BCP 47: Primärkennung aus zwei oder drei Buchstaben,
/// danach nur alphanumerische Untertags. Keine Registry-Prüfung — dafür
/// bräuchte es die IANA-Liste, und die gehört nicht in dieses Crate.
fn is_plausible_lang(l: &str) -> bool {
    let l = l.trim();
    let mut parts = l.split('-');
    let Some(primary) = parts.next() else {
        return false;
    };
    if !(2..=3).contains(&primary.len()) || !primary.chars().all(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    parts.all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric()))
}

fn title<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    match elements(doc).find(|n| n.is_element("title")) {
        None => out.push(
            Finding::fail(
                "document/title-missing",
                "Das Dokument hat kein <title>-Element.",
            )
            .with_severity(Severity::High)
            .with_wcag(["2.4.2"]),
        ),
        Some(t) if subtree_text(t).trim().is_empty() => out.push(
            Finding::fail("document/title-empty", "Das <title>-Element ist leer.")
                .with_severity(Severity::High)
                .with_wcag(["2.4.2"])
                .at(at(t.id())),
        ),
        _ => {}
    }
}

fn viewport<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    for n in elements(doc).filter(|n| n.is_element("meta")) {
        if n.attr("name") != Some("viewport") {
            continue;
        }
        let Some(content) = n.attr("content") else {
            continue;
        };
        let c = content.replace(' ', "");
        let locked = c.contains("user-scalable=no")
            || c.contains("user-scalable=0")
            || c.split(',').any(|p| {
                p.strip_prefix("maximum-scale=")
                    .and_then(|v| v.parse::<f32>().ok())
                    .is_some_and(|v| v < 2.0)
            });
        if locked {
            out.push(
                Finding::fail(
                    "zoom/viewport-locked",
                    "Der Viewport verhindert oder begrenzt das Zoomen.",
                )
                .with_severity(Severity::High)
                .with_wcag(["1.4.4"])
                .at(at(n.id())),
            );
        }
    }
}

// --- Überschriften --------------------------------------------------------

fn headings<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    let mut last = 0u8;
    let mut has_h1 = false;
    let mut any = false;

    for n in elements(doc) {
        let Some(level) = heading_level(n.local_name()) else {
            continue;
        };
        any = true;
        if level == 1 {
            has_h1 = true;
        }
        if subtree_text(n).trim().is_empty() && !n.has_attr("aria-label") {
            out.push(
                Finding::fail("headings/empty", "Die Überschrift hat keinen Text.")
                    .with_severity(Severity::Medium)
                    .with_wcag(["1.3.1", "2.4.6"])
                    .at(at(n.id())),
            );
        }
        if last > 0 && level > last + 1 {
            out.push(
                Finding::fail(
                    "headings/skip-level",
                    format!("Die Gliederung springt von h{last} auf h{level}."),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["1.3.1"])
                .at(at(n.id())),
            );
        }
        last = level;
    }

    if any && !has_h1 {
        out.push(
            Finding::fail(
                "headings/h1-missing",
                "Das Dokument hat keine h1-Überschrift.",
            )
            .with_severity(Severity::Medium)
            .with_wcag(["1.3.1"]),
        );
    }
}

// --- Bilder ---------------------------------------------------------------

fn images<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    for n in elements(doc).filter(|n| n.is_element("img")) {
        match n.attr("alt") {
            None => out.push(
                Finding::fail("images/alt-missing", "Das Bild hat kein alt-Attribut.")
                    .with_severity(Severity::High)
                    .with_wcag(["1.1.1"])
                    .at(at(n.id())),
            ),
            Some(alt) if suspicious_alt(alt) => out.push(
                // Heuristisch: der Text ist da, aber vermutlich nichtssagend.
                // Deshalb Review, nicht Fail.
                Finding::review(
                    "images/alt-suspicious",
                    format!("Der Alt-Text \"{alt}\" beschreibt das Bild vermutlich nicht."),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["1.1.1"])
                .at(at(n.id())),
            ),
            _ => {}
        }
    }
}

fn suspicious_alt(alt: &str) -> bool {
    let a = alt.trim().to_ascii_lowercase();
    if a.is_empty() {
        // Leeres alt ist die korrekte Auszeichnung für dekorative Bilder.
        return false;
    }
    const ENDINGS: &[&str] = &[".jpg", ".jpeg", ".png", ".gif", ".webp", ".svg", ".avif"];
    const FILLERS: &[&str] = &[
        "bild",
        "image",
        "img",
        "foto",
        "photo",
        "grafik",
        "graphic",
        "icon",
        "logo",
        "picture",
        "spacer",
        "platzhalter",
        "placeholder",
    ];
    ENDINGS.iter().any(|e| a.ends_with(e)) || FILLERS.contains(&a.as_str())
}

// --- Formulare ------------------------------------------------------------

/// Die Label-Zuordnung ist vollständig strukturell entscheidbar: `label[for]`,
/// verschachteltes `<label>`, `aria-label`, `aria-labelledby`, `title`. Dafür
/// braucht es keine Accessible-Name-Berechnung.
fn form_labels<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    let label_targets: HashSet<&str> = elements(doc)
        .filter(|n| n.is_element("label"))
        .filter_map(|n| n.attr("for"))
        .collect();

    for n in elements(doc) {
        let tag = n.local_name();
        if !matches!(tag, "input" | "select" | "textarea") {
            continue;
        }
        let ty = n.attr("type").unwrap_or("text").to_ascii_lowercase();
        if matches!(
            ty.as_str(),
            "hidden" | "submit" | "button" | "reset" | "image"
        ) {
            continue;
        }

        let aria_named = n.attr("aria-label").is_some_and(|v| !v.trim().is_empty())
            || n.has_attr("aria-labelledby");
        let for_labelled = n.attr("id").is_some_and(|id| label_targets.contains(id));
        let wrapped = closest(n, "label").is_some();
        let titled = n.attr("title").is_some_and(|v| !v.trim().is_empty());

        if !(aria_named || for_labelled || wrapped || titled) {
            out.push(
                Finding::fail("forms/label-missing", "Das Eingabefeld hat kein Label.")
                    .with_severity(Severity::Critical)
                    .with_wcag(["1.3.1", "3.3.2", "4.1.2"])
                    .at(at(n.id())),
            );
        } else if n.has_attr("placeholder") && !aria_named && !for_labelled && !wrapped {
            out.push(
                Finding::fail(
                    "forms/placeholder-as-label",
                    "Das Feld nutzt den Platzhalter anstelle eines Labels.",
                )
                .with_severity(Severity::Medium)
                .with_wcag(["3.3.2"])
                .at(at(n.id())),
            );
        }
    }
}

// --- ARIA -----------------------------------------------------------------

fn aria_roles<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let Some(role) = n.attr("role") else { continue };
        for r in role.split_whitespace() {
            if ABSTRACT_ROLES.contains(&r) {
                out.push(
                    Finding::fail(
                        "aria/role-abstract",
                        format!(
                            "\"{r}\" ist eine abstrakte Rolle und darf nicht ausgezeichnet werden."
                        ),
                    )
                    .with_severity(Severity::High)
                    .with_wcag(["4.1.2"])
                    .at(at(n.id())),
                );
            } else if !VALID_ROLES.contains(&r) {
                out.push(
                    Finding::fail(
                        "aria/role-invalid",
                        format!("\"{r}\" ist keine gültige ARIA-Rolle."),
                    )
                    .with_severity(Severity::High)
                    .with_wcag(["4.1.2"])
                    .at(at(n.id())),
                );
            }
        }
    }
}

fn aria_references<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    let ids: HashSet<&str> = elements(doc).filter_map(|n| n.attr("id")).collect();

    for n in elements(doc) {
        for rel in [
            "aria-labelledby",
            "aria-describedby",
            "aria-controls",
            "aria-owns",
        ] {
            let Some(v) = n.attr(rel) else { continue };
            let fehlend: Vec<&str> = v
                .split_whitespace()
                .filter(|id| !ids.contains(id))
                .collect();
            if !fehlend.is_empty() {
                out.push(
                    Finding::fail(
                        "aria/reference-missing",
                        format!(
                            "{rel} verweist auf nicht vorhandene IDs: {}",
                            fehlend.join(", ")
                        ),
                    )
                    .with_severity(Severity::High)
                    .with_wcag(["1.3.1", "4.1.2"])
                    .at(at(n.id())),
                );
            }
        }
    }
}

// --- IDs ------------------------------------------------------------------

fn duplicate_ids<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    let mut seen: HashMap<&str, usize> = HashMap::new();
    for n in elements(doc) {
        let Some(id) = n.attr("id") else { continue };
        if id.is_empty() {
            continue;
        }
        let count = seen.entry(id).or_insert(0);
        *count += 1;
        if *count == 2 {
            out.push(
                Finding::fail(
                    "ids/duplicate",
                    format!("Die ID \"{id}\" kommt mehrfach vor."),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["4.1.1"])
                .at(at(n.id())),
            );
        }
    }
}

// --- Tastatur -------------------------------------------------------------

fn tabindex<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let Some(raw) = n.attr("tabindex") else {
            continue;
        };
        let Ok(value) = raw.trim().parse::<i32>() else {
            continue;
        };
        if value > 0 {
            out.push(
                Finding::fail(
                    "keyboard/positive-tabindex",
                    format!("tabindex=\"{value}\" bricht die natürliche Tabreihenfolge."),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["2.4.3"])
                .at(at(n.id())),
            );
        }
    }
}

/// Fokussierbar und zugleich vor dem Accessibility-Tree versteckt — Nutzer
/// landen mit der Tabtaste auf etwas, das ihnen nicht angesagt wird.
fn hidden_focusable<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if n.attr("aria-hidden") != Some("true") {
            continue;
        }
        let natively_focusable = matches!(
            n.local_name(),
            "a" | "button" | "input" | "select" | "textarea" | "summary" | "iframe"
        ) && !n.has_attr("disabled");
        let tab_focusable = n
            .attr("tabindex")
            .and_then(|t| t.trim().parse::<i32>().ok())
            .is_some_and(|v| v >= 0);

        if natively_focusable || tab_focusable {
            out.push(
                Finding::fail(
                    "keyboard/hidden-focusable",
                    "Das Element ist fokussierbar, aber per aria-hidden versteckt.",
                )
                .with_severity(Severity::High)
                .with_wcag(["1.3.1", "4.1.2"])
                .at(at(n.id())),
            );
        }
    }
}

// --- Listen und Tabellen --------------------------------------------------

fn list_structure<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let tag = n.local_name();
        if !matches!(tag, "ul" | "ol") {
            continue;
        }
        let fremd = n
            .children()
            .filter(|c| c.kind() == a11y_dom::NodeKind::Element)
            .any(|c| !matches!(c.local_name(), "li" | "script" | "template"));
        if fremd {
            out.push(
                Finding::fail(
                    "lists/invalid-structure",
                    format!("<{tag}> enthält direkte Kinder, die kein <li> sind."),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["1.3.1"])
                .at(at(n.id())),
            );
        }
    }
}

fn table_headers<D: Document>(doc: &D, out: &mut Vec<Finding>) {
    for n in elements(doc).filter(|n| n.is_element("table")) {
        // Layouttabellen sind explizit ausgezeichnet und nicht gemeint.
        if matches!(n.attr("role"), Some("presentation") | Some("none")) {
            continue;
        }
        let hat_th = a11y_dom::descendants(n).any(|d| d.is_element("th"));
        if !hat_th {
            out.push(
                Finding::fail(
                    "tables/header-missing",
                    "Die Tabelle hat keine <th>-Kopfzellen.",
                )
                .with_severity(Severity::High)
                .with_wcag(["1.3.1"])
                .at(at(n.id())),
            );
        }
    }
}

/// Alle Tier-1-Regeln.
pub fn rules<D: Document>() -> Vec<StructureRule<D>> {
    macro_rules! rule {
        ($id:literal, $wcag:expr, $sev:expr, $help:literal, $f:path) => {
            StructureRule {
                meta: Meta {
                    id: $id,
                    tier: Tier::Structure,
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
            "document/lang",
            &["3.1.1"],
            Severity::High,
            "Das <html>-Element braucht ein gültiges lang-Attribut.",
            lang
        ),
        rule!(
            "document/title",
            &["2.4.2"],
            Severity::High,
            "Jede Seite braucht einen aussagekräftigen <title>.",
            title
        ),
        rule!(
            "zoom/viewport",
            &["1.4.4"],
            Severity::High,
            "Der Viewport darf Zoomen nicht verhindern.",
            viewport
        ),
        rule!(
            "headings",
            &["1.3.1", "2.4.6"],
            Severity::Medium,
            "Überschriften bilden die Gliederung; Ebenen nicht überspringen.",
            headings
        ),
        rule!(
            "images/alt",
            &["1.1.1"],
            Severity::High,
            "Informative Bilder brauchen einen beschreibenden Alt-Text.",
            images
        ),
        rule!(
            "forms/label",
            &["1.3.1", "3.3.2", "4.1.2"],
            Severity::Critical,
            "Jedes Eingabefeld braucht ein zugeordnetes Label.",
            form_labels
        ),
        rule!(
            "aria/role",
            &["4.1.2"],
            Severity::High,
            "Nur Rollen aus der ARIA-Spezifikation verwenden.",
            aria_roles
        ),
        rule!(
            "aria/reference",
            &["1.3.1", "4.1.2"],
            Severity::High,
            "ARIA-Verweise müssen auf vorhandene IDs zeigen.",
            aria_references
        ),
        rule!(
            "ids/duplicate",
            &["4.1.1"],
            Severity::Medium,
            "IDs müssen im Dokument eindeutig sein.",
            duplicate_ids
        ),
        rule!(
            "keyboard/positive-tabindex",
            &["2.4.3"],
            Severity::Medium,
            "Positive tabindex-Werte brechen die Tabreihenfolge.",
            tabindex
        ),
        rule!(
            "keyboard/hidden-focusable",
            &["1.3.1", "4.1.2"],
            Severity::High,
            "Fokussierbare Elemente dürfen nicht aria-hidden sein.",
            hidden_focusable
        ),
        rule!(
            "lists/structure",
            &["1.3.1"],
            Severity::Medium,
            "<ul> und <ol> dürfen als direkte Kinder nur <li> haben.",
            list_structure
        ),
        rule!(
            "tables/header",
            &["1.3.1"],
            Severity::High,
            "Datentabellen brauchen <th>-Kopfzellen.",
            table_headers
        ),
    ]
}
