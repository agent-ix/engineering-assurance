# Campaign contract generation

The source of these wire types is the authored Quire bundle at
`campaign/contract`, interpreted with the
`engineering_assurance` module.

Regenerate by lifting that bundle with the FCD extraction frontend and running
the FCD target generator over the result. Python cache directories produced by
compile checks are excluded from the retained output.

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

## Generator-owned digests in `semantic-ir.json`

BLOCKED-BY PLAT-1205 (owner: planner titan:QSL:plan): source.digest, package.lockDigest, package.manifestDigest, constructs[].manifestDigest required by FCD semantic-IR schema

The FCD 2.0.0 semantic-IR schema (agent-ix/filament-core-data, `crates/semantic-ir/src/schema.rs`: `SOURCE_MEMBERS`, the package members and `CONSTRUCT_ENTRY_MEMBERS`) requires those members in every lifted document, so regenerating this bundle with the FCD extraction frontend reproduces them. They are generator-owned, not authored in this repository, and no FCD generator CLI is available here. Removing them by hand would make the document invalid under that schema and the next regeneration would restore them. They are removed when the FCD schema stops requiring them and this bundle is regenerated.
