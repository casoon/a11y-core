# a11y-rules

Accessibility-Regeln, generisch über das Dokumentmodell aus
[`a11y-dom`](https://crates.io/crates/a11y-dom). Teil von
[a11y-core](https://github.com/casoon/a11y-core).

Ein Regelbestand, mehrere Oberflächen: Build-Zeit, CI/Crawl und die laufende
Seite — mit identischen Regelkennungen und identischem JSON.

## Nicht gelaufen ist nicht bestanden

`run` läuft mit dem, was der Host liefert, und hält für jede Regel fest, ob sie
laufen konnte. Eine Tier-2-Regel auf einem Host ohne `Semantics` erzeugt keinen
stillen Nicht-Befund, sondern einen Vermerk mit `NotRun::CapabilityMissing`.

```rust
use a11y_dom::Arena;
use a11y_rules::run;

let doc = Arena::builder()
    .open("html").open("body")
        .open("img").attr("src", "logo.png").close()
    .close().close()
    .build();

let report = run(&doc);
assert!(report.findings.iter().any(|f| f.rule_id == "images/alt-missing"));

// Nicht beurteilt: die Tier-2-Regeln, weil dieser Host keine Semantik liefert.
assert_eq!(report.summary.rules_not_run, 4);
```

Mit einem Host, der `Semantics` erfüllt, laufen die über `run_with_semantics`
mit. Die Trennung ist keine Formalie: Regeln, die einen echten Accessible Name
brauchen — Links, Buttons, SVG —, dürfen ohne ihn nicht raten.

## Regeln

**Tier 1** (Struktur): `document/lang`, `document/title`, `zoom/viewport`,
`headings`, `images/alt`, `forms/label`, `aria/role`, `aria/reference`,
`ids/duplicate`, `keyboard/positive-tabindex`, `keyboard/hidden-focusable`,
`lists/structure`, `tables/header`.

**Tier 2** (Semantik): `links/name`, `buttons/name`, `svg/name`,
`links/ambiguous-name`.

## Lizenz

MIT.
