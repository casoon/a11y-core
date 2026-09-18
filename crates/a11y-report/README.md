# a11y-report

Gemeinsames Befund- und Berichtsmodell für Accessibility-Werkzeuge. Teil von
[a11y-core](https://github.com/casoon/a11y-core).

## Zwei Achsen, nicht drei

`Outcome` sagt, *wie sicher* die Aussage ist — `Fail`, `Review`, `Pass`,
`Untested`. `Severity` sagt, *wie schwer* das Problem wiegt.

Eine dritte Achse „certainty" gibt es bewusst nicht: Sie wäre weitgehend
dieselbe Achse doppelt, weil `Fail` ohnehin automatisch festgestellt heißt,
`Review` heuristisch und `Untested` nur manuell beurteilbar. Eine Regel, die
ihre Aussage nur vermuten kann, liefert deshalb `Review` — nicht `Fail` mit
niedriger Gewissheit.

Kein aggregierter Score. Ein einzelner Prozentwert würde genau die
Unterscheidung einebnen, für die es die vier Zustände gibt.

```rust
use a11y_report::{Finding, Location, Report, Severity};

let mut report = Report::new();
report.push(
    Finding::fail("images/alt-missing", "Das Bild besitzt kein alt-Attribut.")
        .with_severity(Severity::High)
        .with_wcag(["1.1.1"])
        .at(Location::file("about/index.html").with_selector("main > img")),
);
let report = report.finish();
assert_eq!(report.summary.fail, 1);
```

## „Lief nicht" ist nicht „bestanden"

`RuleRun` hält fest, ob eine Regel überhaupt laufen konnte. Ohne das liest sich
eine Kontrastprüfung ohne Rendering-Zugriff wie eine bestandene Prüfung — der
häufigste Weg, wie ein Bericht mehr verspricht, als er geprüft hat.

## Lizenz

MIT.
