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
claim beyond the selected bytes. Older source-pinned procedures omit the field
and cannot claim generic source/dependency origin assurance on that basis.
For an opted-in procedure, every caller-selected input must match an authored
exact role and origin. A dynamic prefix cannot add an undeclared selected
input. Source-tree projection is separate: EA derives those implicit inputs
from the verified Git manifest, and Quoin rechecks them at replay.
