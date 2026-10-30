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

BLOCKED-BY <ticket id not assigned to this PR> (owner: planner titan:QSL:plan): the FCD 2.0.0 semantic-IR schema (agent-ix/filament-core-data, `crates/semantic-ir/src/schema.rs`) requires `source.digest`, `package.lockDigest`, `package.manifestDigest` and `constructs[].manifestDigest` in every lifted document (they are listed in its fixed member sets `SOURCE_MEMBERS`, `CONSTRUCT_ENTRY_MEMBERS`), so regenerating this bundle with the FCD extraction frontend reproduces them. They are generator-owned, not authored in this repository, and removing them by hand would make the document invalid under that schema and be overwritten by the next regeneration. They are removed when the FCD schema stops requiring them and this bundle is regenerated.
