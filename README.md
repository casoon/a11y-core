# a11y-core

Gemeinsame Bausteine für Accessibility-Werkzeuge in Rust. Ein Regelbestand
bedient drei Oberflächen mit identischen Regelkennungen und identischem JSON:

| Oberfläche | Werkzeug | Substrat |
|---|---|---|
| Build-Zeit | [astro-post-audit](https://github.com/casoon/astro-post-audit) | statisches HTML aus `dist/` |
| CI / Crawl | [auditmysite](https://github.com/casoon/auditmysite) | Chrome via CDP, nativer Accessibility-Tree |
| laufende Seite | [liveaudit](https://github.com/casoon/liveaudit) | DOM der Seite, über WASM |

## Crates

| Crate | Zweck | Stand |
|---|---|---|
| [`a11y-report`](crates/a11y-report) | Befund- und Berichtsmodell, JSON-Vertrag | erste Fassung |
| `a11y-dom` | Dokumentmodell-Abstraktion, generisch über den Baum des Hosts | geplant |
| `accname` | WAI-ARIA Accessible Name und Role Computation | geplant |
| `a11y-rules` | die Regeln selbst, generisch über Fähigkeits-Tiers | geplant |
| `a11y-conformance` | geteiltes Fixture-Korpus gegen Auseinanderlaufen | geplant |

## Zwei Achsen, nicht drei

`Outcome` sagt, *wie sicher* die Aussage ist — `Fail`, `Review`, `Pass`,
`Untested`. `Severity` sagt, *wie schwer* das Problem wiegt — `Low` bis
`Critical`.

Eine dritte Achse „certainty" gibt es bewusst nicht: Sie wäre weitgehend
dieselbe Achse doppelt, weil `Fail` ohnehin automatisch festgestellt heißt,
`Review` heuristisch und `Untested` nur manuell beurteilbar. Eine Regel, die
ihre Aussage nur vermuten kann, liefert deshalb `Review` — nicht `Fail` mit
niedriger Gewissheit.

Kein aggregierter Score. Ein einzelner Prozentwert würde genau die
Unterscheidung einebnen, für die es die vier Zustände gibt.

## „Lief nicht" ist nicht „bestanden"

`RuleRun` hält fest, ob eine Regel überhaupt laufen konnte. Ohne das liest sich
eine Kontrastprüfung ohne Rendering-Zugriff wie eine bestandene Prüfung — der
häufigste Weg, wie ein Bericht mehr verspricht, als er geprüft hat.

```rust
use a11y_report::{NotRun, Report, RuleRun};

let mut report = Report::new();
report.record(
    RuleRun::not_run("contrast/text", NotRun::CapabilityMissing)
        .with_reason("statische Analyse liefert keine Rendering-Werte"),
);
```

## Lizenz

MIT.
