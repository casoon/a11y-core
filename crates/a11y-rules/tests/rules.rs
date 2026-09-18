//! Regeltests gegen die Referenz-Arena.

use a11y_dom::{elements, subtree_text, Arena, Document, NameSource, Node, Semantics};
use a11y_report::{Outcome, Report};
use a11y_rules::{run, run_with_semantics};

/// Ein Tier-2-Host für die Tests.
///
/// **Kein Ersatz für `accname`.** Diese Näherung — `aria-label`, dann `alt`,
/// dann Teilbaumtext, dann `title` — reicht für die Testfälle hier, liegt aber
/// in genau den Fällen falsch, für die es das eigene Crate braucht:
/// `aria-labelledby`-Ketten, versteckte Teilbäume, eingebettete Bilder. Sobald
/// `accname` steht, ersetzt es das hier.
struct MitSemantik(Arena);

impl Document for MitSemantik {
    type N<'a>
        = <Arena as Document>::N<'a>
    where
        Self: 'a;

    fn root(&self) -> Self::N<'_> {
        self.0.root()
    }
}

impl Semantics for MitSemantik {
    fn role(&self, node: Self::N<'_>) -> Option<String> {
        node.attr("role")
            .map(str::to_string)
            .or_else(|| match node.local_name() {
                "a" if node.has_attr("href") => Some("link".into()),
                "button" => Some("button".into()),
                "img" => Some("img".into()),
                _ => None,
            })
    }

    fn accessible_name(&self, node: Self::N<'_>) -> Option<String> {
        let kandidat = node
            .attr("aria-label")
            .map(str::to_string)
            .or_else(|| node.attr("alt").map(str::to_string))
            .unwrap_or_else(|| subtree_text(node));
        let kandidat = if kandidat.trim().is_empty() {
            node.attr("title").unwrap_or("").to_string()
        } else {
            kandidat
        };
        (!kandidat.trim().is_empty()).then(|| kandidat.trim().to_string())
    }

    fn name_source(&self, node: Self::N<'_>) -> Option<NameSource> {
        node.has_attr("aria-label").then_some(NameSource::AriaLabel)
    }

    fn is_ignored(&self, node: Self::N<'_>) -> bool {
        node.attr("aria-hidden") == Some("true")
    }
}

fn ids(r: &Report) -> Vec<&str> {
    r.findings.iter().map(|f| f.rule_id.as_str()).collect()
}

fn hat(r: &Report, id: &str) -> bool {
    r.findings.iter().any(|f| f.rule_id == id)
}

/// Ein minimal korrektes Dokument, von dem die Einzeltests abweichen.
fn sauber() -> a11y_dom::ArenaBuilder {
    Arena::builder()
        .open("html")
        .attr("lang", "de")
        .open("head")
        .open("title")
        .text("Seite")
        .close()
        .close()
}

#[test]
fn sauberes_dokument_erzeugt_nur_h1_hinweis_nicht() {
    let doc = sauber()
        .open("body")
        .open("h1")
        .text("Titel")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(r.findings.is_empty(), "unerwartet: {:?}", ids(&r));
}

#[test]
fn fehlendes_lang_und_title() {
    let doc = Arena::builder()
        .open("html")
        .open("body")
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(hat(&r, "document/lang-missing"));
    assert!(hat(&r, "document/title-missing"));
}

#[test]
fn unplausibler_sprachcode() {
    for code in ["", "d", "deutsch-", "1de"] {
        let doc = Arena::builder()
            .open("html")
            .attr("lang", code)
            .open("head")
            .open("title")
            .text("x")
            .close()
            .close()
            .close()
            .build();
        let r = run(&doc);
        assert!(
            hat(&r, "document/lang-invalid"),
            "Code {code:?} haette auffallen muessen"
        );
    }
    // Gueltige Formen duerfen nicht anschlagen.
    for code in ["de", "de-DE", "en", "zh-Hant-TW"] {
        let doc = Arena::builder()
            .open("html")
            .attr("lang", code)
            .open("head")
            .open("title")
            .text("x")
            .close()
            .close()
            .close()
            .build();
        let r = run(&doc);
        assert!(
            !hat(&r, "document/lang-invalid"),
            "Code {code:?} ist gueltig"
        );
    }
}

#[test]
fn bild_ohne_alt_ist_fail_verdaechtiges_alt_ist_review() {
    let doc = sauber()
        .open("body")
        .open("img")
        .attr("src", "a.png")
        .close()
        .open("img")
        .attr("src", "b.png")
        .attr("alt", "b.png")
        .close()
        .open("img")
        .attr("src", "c.png")
        .attr("alt", "Ein Hund im Schnee")
        .close()
        .open("img")
        .attr("src", "d.png")
        .attr("alt", "")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);

    let fehlend = r
        .findings
        .iter()
        .find(|f| f.rule_id == "images/alt-missing")
        .unwrap();
    assert_eq!(fehlend.outcome, Outcome::Fail);

    let verdaechtig = r
        .findings
        .iter()
        .find(|f| f.rule_id == "images/alt-suspicious")
        .unwrap();
    assert_eq!(
        verdaechtig.outcome,
        Outcome::Review,
        "heuristische Regeln liefern Review, nicht Fail"
    );

    // Genau einmal je Regel: der gute Alt-Text und das leere alt schlagen nicht an.
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.rule_id.starts_with("images/"))
            .count(),
        2
    );
}

#[test]
fn ueberschriften_luecke_und_leere_ueberschrift() {
    let doc = sauber()
        .open("body")
        .open("h1")
        .text("A")
        .close()
        .open("h3")
        .text("B")
        .close()
        .open("h4")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(hat(&r, "headings/skip-level"));
    assert!(hat(&r, "headings/empty"));
    assert!(!hat(&r, "headings/h1-missing"));
}

#[test]
fn label_zuordnung_ueber_alle_vier_wege() {
    let doc = sauber()
        .open("body")
        // 1. label[for]
        .open("label")
        .attr("for", "a")
        .text("A")
        .close()
        .open("input")
        .attr("id", "a")
        .close()
        // 2. verschachtelt
        .open("label")
        .text("B")
        .open("input")
        .attr("id", "b")
        .close()
        .close()
        // 3. aria-label
        .open("input")
        .attr("id", "c")
        .attr("aria-label", "C")
        .close()
        // 4. gar nichts
        .open("input")
        .attr("id", "d")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.rule_id == "forms/label-missing")
            .count(),
        1,
        "nur das vierte Feld ist unbeschriftet: {:?}",
        ids(&r)
    );
}

#[test]
fn placeholder_ersetzt_kein_label() {
    let doc = sauber()
        .open("body")
        .open("input")
        .attr("id", "a")
        .attr("title", "Suche")
        .attr("placeholder", "Suchen…")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    // title zaehlt als Label, also kein label-missing - aber der Platzhalter
    // bleibt ein eigener Befund.
    assert!(!hat(&r, "forms/label-missing"));
    assert!(hat(&r, "forms/placeholder-as-label"));
}

#[test]
fn versteckte_felder_werden_uebergangen() {
    let doc = sauber()
        .open("body")
        .open("input")
        .attr("type", "hidden")
        .attr("name", "csrf")
        .close()
        .open("input")
        .attr("type", "submit")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(!hat(&r, "forms/label-missing"));
}

#[test]
fn ungueltige_und_abstrakte_rollen() {
    let doc = sauber()
        .open("body")
        .open("div")
        .attr("role", "buton")
        .close()
        .open("div")
        .attr("role", "widget")
        .close()
        .open("div")
        .attr("role", "button")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(hat(&r, "aria/role-invalid"));
    assert!(hat(&r, "aria/role-abstract"));
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.rule_id.starts_with("aria/role"))
            .count(),
        2
    );
}

#[test]
fn aria_verweise_auf_fehlende_ids() {
    let doc = sauber()
        .open("body")
        .open("h2")
        .attr("id", "da")
        .text("Da")
        .close()
        .open("div")
        .attr("aria-labelledby", "da fehlt")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    let f = r
        .findings
        .iter()
        .find(|f| f.rule_id == "aria/reference-missing")
        .unwrap();
    assert!(f.message.contains("fehlt"), "{}", f.message);
    assert!(
        !f.message.contains(" da"),
        "vorhandene ID darf nicht gemeldet werden: {}",
        f.message
    );
}

#[test]
fn doppelte_ids_melden_nur_den_zweiten_treffer() {
    let doc = sauber()
        .open("body")
        .open("div")
        .attr("id", "x")
        .close()
        .open("div")
        .attr("id", "x")
        .close()
        .open("div")
        .attr("id", "x")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.rule_id == "ids/duplicate")
            .count(),
        1
    );
}

#[test]
fn positiver_tabindex_und_verstecktes_fokussierbares() {
    let doc = sauber()
        .open("body")
        .open("div")
        .attr("tabindex", "4")
        .close()
        .open("div")
        .attr("tabindex", "0")
        .close()
        .open("a")
        .attr("href", "/x")
        .attr("aria-hidden", "true")
        .text("X")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.rule_id == "keyboard/positive-tabindex")
            .count(),
        1
    );
    assert!(hat(&r, "keyboard/hidden-focusable"));
}

#[test]
fn viewport_sperre() {
    for content in [
        "width=device-width, user-scalable=no",
        "width=device-width,maximum-scale=1.0",
    ] {
        let doc = Arena::builder()
            .open("html")
            .attr("lang", "de")
            .open("head")
            .open("title")
            .text("x")
            .close()
            .open("meta")
            .attr("name", "viewport")
            .attr("content", content)
            .close()
            .close()
            .close()
            .build();
        assert!(hat(&run(&doc), "zoom/viewport-locked"), "{content}");
    }
    // Erlaubter Viewport
    let doc = Arena::builder()
        .open("html")
        .attr("lang", "de")
        .open("head")
        .open("title")
        .text("x")
        .close()
        .open("meta")
        .attr("name", "viewport")
        .attr("content", "width=device-width, initial-scale=1")
        .close()
        .close()
        .close()
        .build();
    assert!(!hat(&run(&doc), "zoom/viewport-locked"));
}

#[test]
fn listen_und_tabellen() {
    let doc = sauber()
        .open("body")
        .open("ul")
        .open("div")
        .text("falsch")
        .close()
        .close()
        .open("table")
        .open("tr")
        .open("td")
        .text("x")
        .close()
        .close()
        .close()
        .open("table")
        .attr("role", "presentation")
        .open("tr")
        .open("td")
        .text("Layout")
        .close()
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(hat(&r, "lists/invalid-structure"));
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.rule_id == "tables/header-missing")
            .count(),
        1,
        "role=presentation ist eine Layouttabelle und gemeint"
    );
}

// --- Tier-Mechanik --------------------------------------------------------

#[test]
fn ohne_semantik_werden_tier2_regeln_als_nicht_gelaufen_vermerkt() {
    let doc = sauber()
        .open("body")
        .open("a")
        .attr("href", "/x")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);

    assert_eq!(r.summary.rules_not_run, 4);
    assert!(
        !hat(&r, "links/name-missing"),
        "ohne Accessible Name darf die Regel nicht raten"
    );

    let vermerk = r
        .rule_runs
        .iter()
        .find(|x| x.rule_id == "links/name")
        .unwrap();
    assert!(!vermerk.did_run());
    assert_eq!(
        vermerk.not_run,
        Some(a11y_report::NotRun::CapabilityMissing)
    );
}

#[test]
fn mit_semantik_laufen_tier2_regeln_mit() {
    let doc = MitSemantik(
        sauber()
            .open("body")
            .open("a")
            .attr("href", "/x")
            .close()
            .open("a")
            .attr("href", "/y")
            .text("Mehr")
            .close()
            .open("button")
            .close()
            .close()
            .close()
            .build(),
    );
    let r = run_with_semantics(&doc);

    assert_eq!(r.summary.rules_not_run, 0);
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.rule_id == "links/name-missing")
            .count(),
        1
    );
    assert!(hat(&r, "buttons/name-missing"));
}

#[test]
fn gleicher_linktext_verschiedene_ziele() {
    let doc = MitSemantik(
        sauber()
            .open("body")
            .open("a")
            .attr("href", "/a")
            .text("Mehr")
            .close()
            .open("a")
            .attr("href", "/b")
            .text("Mehr")
            .close()
            .open("a")
            .attr("href", "/c")
            .text("Zum Bericht")
            .close()
            .close()
            .close()
            .build(),
    );
    let r = run_with_semantics(&doc);
    let treffer: Vec<_> = r
        .findings
        .iter()
        .filter(|f| f.rule_id == "links/ambiguous-name")
        .collect();
    assert_eq!(treffer.len(), 2, "beide mehrdeutigen Links werden markiert");
    assert_eq!(treffer[0].outcome, Outcome::Review);
}

#[test]
fn gleicher_linktext_gleiches_ziel_ist_in_ordnung() {
    let doc = MitSemantik(
        sauber()
            .open("body")
            .open("a")
            .attr("href", "/a")
            .text("Mehr")
            .close()
            .open("a")
            .attr("href", "/a")
            .text("Mehr")
            .close()
            .close()
            .close()
            .build(),
    );
    assert!(!hat(&run_with_semantics(&doc), "links/ambiguous-name"));
}

#[test]
fn aria_hidden_elemente_bleiben_bei_tier2_aussen_vor() {
    let doc = MitSemantik(
        sauber()
            .open("body")
            .open("a")
            .attr("href", "/x")
            .attr("aria-hidden", "true")
            .close()
            .close()
            .close()
            .build(),
    );
    assert!(!hat(&run_with_semantics(&doc), "links/name-missing"));
}

#[test]
fn jede_regel_hinterlaesst_genau_einen_ausfuehrungsvermerk() {
    let doc = sauber().open("body").close().close().build();
    let r = run(&doc);
    let mut kennungen: Vec<&str> = r.rule_runs.iter().map(|x| x.rule_id.as_str()).collect();
    let anzahl = kennungen.len();
    kennungen.sort_unstable();
    kennungen.dedup();
    assert_eq!(kennungen.len(), anzahl, "doppelte Vermerke");
    assert_eq!(anzahl, 13 + 4, "13 Tier-1- plus 4 Tier-2-Regeln");
}

#[test]
fn befunde_tragen_wcag_kriterien_und_eine_verortung() {
    let doc = sauber()
        .open("body")
        .open("img")
        .attr("src", "a.png")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    let f = r
        .findings
        .iter()
        .find(|f| f.rule_id == "images/alt-missing")
        .unwrap();
    assert_eq!(f.wcag, vec!["1.1.1"]);
    assert!(f.location.node.is_some());

    // Die Verortung laesst sich zum Knoten zurueckaufloesen.
    let id: u32 = f.location.node.as_ref().unwrap().parse().unwrap();
    let knoten = doc.get(a11y_dom::NodeId(id)).unwrap();
    assert_eq!(knoten.local_name(), "img");
    let _ = elements(&doc).count();
}
