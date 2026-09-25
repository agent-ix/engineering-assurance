# Campaign contract generation

The source of these wire types is the authored Quire bundle at
`campaign/contract`, interpreted with the
`engineering_assurance` module and Quire CLI 0.33.0 (engine 0.47.1).
The recorded generator source is `agent-ix/filament-core-data` commit
`9ef44b34dc80c825f625e04e5264d6c0c0952e31`.

From that FCD checkout, lift with `cargo +1.98.1 run --locked -p
agent-ix-extraction-frontend --bin extraction-frontend -- lift --bundle
<EA>/campaign/contract --module
<EA>/engineering_assurance --out <scratch>/semantic-ir.json`. Generate
with `node scripts/spec-to-targets.mjs <scratch>/semantic-ir.json
<scratch>/generated --compile`, then run `cargo fmt --manifest-path
<scratch>/generated/rust/Cargo.toml`. The committed `semantic-ir.json` and
five target directories must match those results byte for byte. Python cache
directories produced by compile checks are excluded from the retained output.

The 0.5.0 candidate compatibility matrix hashes the retained semantic IR and
JSON Schema documents. The 0.4.1 accepted matrix is unchanged.

`MeasurementProcedure.inputOrigins` is an optional migration field for exact
producer-input provenance. When present, EA requires one declaration for every
explicit input role. `source_file` fixes a campaign repository and tracked
path; `dependency` fixes the upstream member and output role while the attempt
index is selected and retained at run time; `selected_bytes` makes no origin
claim beyond the selected bytes. Older source-pinned procedures omit the field
and cannot claim generic source/dependency origin assurance on that basis.
For an opted-in procedure, every caller-selected input must match an authored
exact role and origin. A dynamic prefix cannot add an undeclared selected
input. Source-tree projection is separate: EA derives those implicit inputs
from the verified Git manifest, and Quoin rechecks them at replay.
