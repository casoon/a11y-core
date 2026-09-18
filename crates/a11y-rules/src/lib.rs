//! Accessibility-Regeln, generisch über das Dokumentmodell.
//!
//! Ein Regelbestand, drei Oberflächen: Build-Zeit, CI/Crawl und die laufende
//! Seite. Welche Regeln laufen können, hängt davon ab, welche
//! [`Tier`](a11y_dom::Tier)s der Host bedient.
//!
//! # Nicht gelaufen ist nicht bestanden
//!
//! [`run`] läuft mit dem, was da ist, und hält für jede Regel fest, ob sie
//! laufen konnte. Eine Tier-2-Regel auf einem Host ohne
//! [`Semantics`](a11y_dom::Semantics) erzeugt keinen stillen Nicht-Befund,
//! sondern einen Vermerk mit `NotRun::CapabilityMissing`.
//!
//! ```
//! use a11y_dom::Arena;
//! use a11y_rules::run;
//!
//! let doc = Arena::builder()
//!     .open("html")
//!         .open("body")
//!             .open("img").attr("src", "logo.png").close()
//!         .close()
//!     .close()
//!     .build();
//!
//! let report = run(&doc);
//!
//! // Gefunden: kein lang, kein title, kein alt.
//! assert!(report.findings.iter().any(|f| f.rule_id == "images/alt-missing"));
//! assert!(report.findings.iter().any(|f| f.rule_id == "document/lang-missing"));
//!
//! // Nicht beurteilt: die Tier-2-Regeln, weil dieser Host keine Semantik liefert.
//! assert_eq!(report.summary.rules_not_run, 4);
//! ```
//!
//! Mit einem Host, der [`Semantics`](a11y_dom::Semantics) erfüllt, laufen die
//! über [`run_with_semantics`] mit.

#![forbid(unsafe_code)]

mod registry;
mod semantics;
mod structure;

pub use registry::{Meta, SemanticsRule, StructureRule};

use a11y_dom::{Document, Semantics};
use a11y_report::{Finding, NotRun, Report, RuleRun};

/// Alle Tier-1-Regeln.
pub fn structure_rules<D: Document>() -> Vec<StructureRule<D>> {
    structure::rules()
}

/// Alle Tier-2-Regeln.
pub fn semantics_rules<D: Semantics>() -> Vec<SemanticsRule<D>> {
    semantics::rules()
}

/// Nur die Kennungen der Tier-2-Regeln — für Hosts, die sie nicht ausführen
/// können, aber trotzdem benennen müssen.
fn semantics_rule_ids() -> &'static [&'static str] {
    &[
        "links/name",
        "buttons/name",
        "svg/name",
        "links/ambiguous-name",
    ]
}

fn run_structure<D: Document>(doc: &D, report: &mut Report) {
    for rule in structure_rules::<D>() {
        let mut out: Vec<Finding> = Vec::new();
        (rule.run)(doc, &mut out);
        report.record(RuleRun::ran(rule.meta.id, out.len()));
        report.extend(out);
    }
}

/// Prüft ein Dokument, das nur Struktur liefert.
///
/// Tier-2-Regeln werden mit `NotRun::CapabilityMissing` vermerkt, nicht
/// übergangen — der Bericht sagt damit aus, was er *nicht* geprüft hat.
pub fn run<D: Document>(doc: &D) -> Report {
    let mut report = Report::new();
    run_structure(doc, &mut report);
    for id in semantics_rule_ids() {
        report.record(
            RuleRun::not_run(*id, NotRun::CapabilityMissing)
                .with_reason("Host liefert keine Rolle und keinen Accessible Name"),
        );
    }
    report.finish()
}

/// Prüft ein Dokument, das zusätzlich Rolle und Accessible Name liefert.
pub fn run_with_semantics<D: Semantics>(doc: &D) -> Report {
    let mut report = Report::new();
    run_structure(doc, &mut report);
    for rule in semantics_rules::<D>() {
        let mut out: Vec<Finding> = Vec::new();
        (rule.run)(doc, &mut out);
        report.record(RuleRun::ran(rule.meta.id, out.len()));
        report.extend(out);
    }
    report.finish()
}
