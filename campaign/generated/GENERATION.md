# Campaign contract generation

The source of these wire types is the authored Filament bundle at
`campaign/contract`, interpreted with the
`engineering_assurance` module.

Regenerate by lifting that bundle with the FCD extraction frontend and running
the FCD target generator over the result. The output is generated, never
hand-edited. It was last generated with filament-core-data at commit d7a9b7b6
(`<ea>` is this repository, run from the fcd checkout):

```
cargo run --locked -p agent-ix-extraction-frontend --bin extraction-frontend -- lift --bundle <ea>/campaign/contract --module <ea>/engineering_assurance --out <ea>/campaign/generated/semantic-ir.json
node src/compiler/cli.mjs generate --ir <ea>/campaign/generated/semantic-ir.json --target rust --out-root <ea>/campaign/generated/rust
node src/compiler/cli.mjs generate --ir <ea>/campaign/generated/semantic-ir.json --target json-schema --out-root <ea>/campaign/generated/json-schema
```

The lift writes a `semantic-ir.json.diagnostics.json` beside the IR; it is not
retained. Python cache directories produced by compile checks are excluded from
the retained output.

`MeasurementProcedure.inputOrigins` is an optional migration field for exact
producer-input provenance. When present, EA requires one declaration for every
explicit input role. `source_file` fixes a campaign repository and tracked
path; `dependency` fixes the upstream member and output role while the attempt
index is selected and retained at run time; `selected_bytes` makes no origin
claim beyond the selected bytes. Older procedures omit the field and cannot claim generic
source/dependency origin assurance on that basis.
For an opted-in procedure, every caller-selected input must match an authored
exact role and origin. A dynamic prefix cannot add an undeclared selected
input. Source-tree projection is separate: EA derives those implicit inputs
from the Git tree manifest, and Quoin rechecks them at replay.
