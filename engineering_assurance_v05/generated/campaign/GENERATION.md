# Campaign contract generation

The source of these wire types is the authored Quire bundle at
`engineering_assurance_v05/contracts/campaign`, interpreted with the
`engineering_assurance_v05` module and Quire CLI 0.33.0 (engine 0.47.1).
The recorded generator source is `agent-ix/filament-core-data` commit
`9ef44b34dc80c825f625e04e5264d6c0c0952e31`.

From that FCD checkout, lift with `cargo +1.98.1 run --locked -p
agent-ix-extraction-frontend --bin extraction-frontend -- lift --bundle
<EA>/engineering_assurance_v05/contracts/campaign --module
<EA>/engineering_assurance_v05 --out <scratch>/semantic-ir.json`. Generate
with `node scripts/spec-to-targets.mjs <scratch>/semantic-ir.json
<scratch>/generated --compile`, then run `cargo fmt --manifest-path
<scratch>/generated/rust/Cargo.toml`. The committed `semantic-ir.json` and
five target directories must match those results byte for byte. Python cache
directories produced by compile checks are excluded from the retained output.

The 0.5.0 candidate compatibility matrix hashes the retained semantic IR and
JSON Schema documents. The 0.4.1 accepted matrix is unchanged.
