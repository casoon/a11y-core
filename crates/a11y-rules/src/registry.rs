//! Regeln als Funktionszeiger, nach Tier getrennt registriert.
//!
//! Keine Trait-Objekte: Eine Regel ist ein `fn`, die Registry ist ein Slice.
//! Das monomorphisiert pro Host, allokiert nichts pro Regel und hält die
//! Tier-Grenze im Typsystem — eine Tier-2-Regel kann gar nicht erst mit einem
//! Host aufgerufen werden, der [`Semantics`] nicht erfüllt.
//!
//! [`Semantics`]: a11y_dom::Semantics

use a11y_dom::{Document, Semantics, Tier};
use a11y_report::{Finding, Severity};

/// Was über eine Regel unabhängig vom Host feststeht.
#[derive(Debug, Clone, Copy)]
pub struct Meta {
    /// Stabile Kennung, über alle drei Oberflächen identisch.
    pub id: &'static str,
    /// Welche Datenschicht die Regel braucht.
    pub tier: Tier,
    /// WCAG-Erfolgskriterien, z. B. `["1.1.1"]`.
    pub wcag: &'static [&'static str],
    /// Vorgabeschwere. Einzelne Befunde dürfen davon abweichen.
    pub severity: Severity,
    pub help: &'static str,
}

/// Eine Regel auf [`Tier::Structure`] — Tags, Attribute, Text, Hierarchie.
pub struct StructureRule<D: Document> {
    pub meta: Meta,
    pub run: fn(&D, &mut Vec<Finding>),
}

impl<D: Document> Clone for StructureRule<D> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<D: Document> Copy for StructureRule<D> {}

/// Eine Regel auf [`Tier::Semantics`] — braucht Rolle und Accessible Name.
pub struct SemanticsRule<D: Semantics> {
    pub meta: Meta,
    pub run: fn(&D, &mut Vec<Finding>),
}

impl<D: Semantics> Clone for SemanticsRule<D> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<D: Semantics> Copy for SemanticsRule<D> {}
