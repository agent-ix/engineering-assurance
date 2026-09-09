---
id: SR-058
title: "Base review of the assurance-contract registry model and census"
type: SpecReview
analysis: base
scope: "ADR-002 assurance-contract census; FR-015-AC-5..AC-7; TC-119, TC-120, TC-122; TASK-017"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: reviews
---

## Summary

This focused QUOIN base review checks whether TASK-017 can represent the real
AssuranceProfile and MeasurementPlan population before Rust implementation.
The diagnostic census inspected the exact default-branch trees of all 294
non-archived repositories visible in the `agent-ix` organization on
2026-09-09. It found AP/MP instances in 13 external repositories and 53 active
artifact paths. Engineering Assurance's own schemas and two package skeletons
belong to the provider binding, not the external consumer registry.

The scan is proposal evidence, not the release-scope authority. The committed
registry becomes authoritative only after the named human owner accepts its
population, classifications, and candidate revisions.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-118 | high | Closed in the specification: FR-015 required one unique row per repository while saying each row classified one artifact. Real repositories contain as many as 22 AP/MP artifacts, so the old model could only omit artifacts or violate its duplicate-repository rule. Repository entries now bind one candidate commit and contain one or more independently classified artifact entries with unique repository-relative paths. | FR-015 behavior and AC-7; TC-122; TASK-017 | wrong-requirement |
| FND-119 | medium | Closed in the specification: ADR-002's named current-shape census omitted `ix://agent-ix/quire-code-rs`, whose default branch contains `spec/assurance/MP-001-graph-quality-observation.md`. The named list now includes it. | ADR-002 assurance-contract census; `quire-code-rs@e1b7fc3` blob `213fbeed` | missing-requirement |
| FND-120 | high | Closed in the specification: “owner-reviewed” had no encoded acceptance state or attribution rule, allowing an agent-prepared or half-attributed registry to govern by assertion. FR-015 now permits only unattributed `pending` or human-attributed `accepted`, binds acceptance to the exact canonical population digest, and withholds snapshots for every other state. | FR-015 behavior, CON-4, CON-5, and AC-7; TC-122; TASK-017 | missing-requirement |
| FND-121 | high | Closed in the specification: a registry “digest over its canonical bytes” was self-referential once the digest and acceptance were fields in those bytes. Registry/v1 now defines a canonical `population` digest, owner acceptance references that digest, and each snapshot separately binds the exact registry input-byte digest to detect any within-run mutation. | FR-015 behavior and AC-7; TC-122 | wrong-requirement |
| FND-122 | high | Closed in the specification: the initial review draft proposed classifying Engineering Assurance's own skeletons in a registry stored in that repository, which would require the registry to name the not-yet-existing commit that contains itself. FR-015 and TC-119 now keep the provider candidate, schemas, and skeletons in a separate snapshot binding and prohibit the provider repository from the external consumer registry. | FR-015 behavior and AC-5; TC-119; TASK-017 | wrong-requirement |
| FND-123 | medium | Closed in this review: repository counts and candidate commits alone did not identify the exact bytes submitted for owner acceptance. The proposed population now enumerates all 53 paths with full Git blob provenance and SHA-256 content digests. | Proposed owner registry population; exact active artifact bindings | correct-requirement-no-evidence |

## Proposed owner registry population

The following external population is the complete exact-tree result. Counts are
artifact entries, not repository rows. Unless the owner records another
classification, the proposed classification is `active` for these 53
instances.

| Canonical repository | Candidate commit | AP | MP |
| --- | --- | ---: | ---: |
| `ix://agent-ix/qa-corpus` | `820c0e76ff3b4395c6b52ccf863de51446fd5e37` | 1 | 2 |
| `ix://agent-ix/quire-analyze` | `5f37678928aea451447d1ee6d42deefb95cb2f5a` | 1 | 1 |
| `ix://agent-ix/quire-code-rs` | `e1b7fc303f3df3c9e3673358ce990525b7ff321e` | 0 | 1 |
| `ix://agent-ix/quire-contract-codegen` | `240fad84a9565ab723ba9844e18faea4e5d96f66` | 1 | 1 |
| `ix://agent-ix/quire-contract-ir` | `690bde7f2dc58662cf9ff0595c2c0e3b17107c6f` | 1 | 1 |
| `ix://agent-ix/quire-contract-runtime` | `7caefac3b7181d7665caa8f83c6694134fac63ae` | 1 | 1 |
| `ix://agent-ix/quire-rs` | `8b8020e665c61a11bc74f3f23b84617c80c0c442` | 1 | 8 |
| `ix://agent-ix/quire-verification` | `a57a88d9b259d7c48e95d375da0a8b01b3be7c0d` | 1 | 1 |
| `ix://agent-ix/quoin` | `7072d65db7e236e819d99be42e094ba548e9496a` | 1 | 21 |
| `ix://agent-ix/tl-mltl` | `9b3b68bc78589fa32805e105d9f8d97a0ad280b3` | 1 | 1 |
| `ix://agent-ix/tl-parse` | `103e6fd1573fa87534efec206f5d3e3b9c3f9d82` | 1 | 1 |
| `ix://agent-ix/tl-rewrite` | `1ccab45dc64d5a344740256c5a91c2e10555decc` | 1 | 1 |
| `ix://agent-ix/tl-syntax` | `26b801d4a68ebfe720062cfdb3c66b070ab60e92` | 1 | 1 |

### Exact active artifact bindings

Every proposed active entry below is bound by its repository commit in the table above, its repository-relative path, its Git blob provenance, and its SHA-256 content digest.

#### `ix://agent-ix/qa-corpus`

- `assurance/AP-201-controlled-corpus.md` — blob `48f6618c2c51f98d7dd6f7a4860127683adb9f59`; SHA-256 `11bf2b4f983bbbf491d2901db742f5610d363d39dd476b4092c1955bba8fd8a6`
- `assurance/MP-201-gap-count.md` — blob `0b11a0390d3d168316aafd4fbf50f9832197ae18`; SHA-256 `3f91245c81ea4942b40c6513570df8d15cc6ca067ee1e8665e12e108d49bf2d2`
- `assurance/MP-202-detection-recall.md` — blob `6ebbe6cd21ecd944abbffd53ac2c185bb2cc1950`; SHA-256 `a91b87cc35416dc7582d1f2f3975b4e20ef6f9a57ce99f986a81b8061df8553f`

#### `ix://agent-ix/quire-analyze`

- `spec/assurance/AP-001-analyze-release.md` — blob `421d8175197a48fab99d83217d2253867420c617`; SHA-256 `af0a8d8c02f9531801486259c7d91777fddb48a6aa4b82c365b61c6f42a6bf7e`
- `spec/assurance/MP-001-analyze-measurements.md` — blob `1f8f5f5764d5ba95c8ed77da1389b7fc692d4b8c`; SHA-256 `9f7dda8f92cd483e5674691882ddc9bab8b5ee9a6ad90f11f06d70f94446a8c4`

#### `ix://agent-ix/quire-code-rs`

- `spec/assurance/MP-001-graph-quality-observation.md` — blob `213fbeeda28142450920c297d04d109a068172c0`; SHA-256 `e0fa69054b7662569fb8a7d34e273f691213015783e6395271b3786736b4c1f4`

#### `ix://agent-ix/quire-contract-codegen`

- `spec/assurance/AP-001-codegen-release.md` — blob `6852e6836f85e05c7f531a8a8697eaf9c2e2da5a`; SHA-256 `0fe2b0fa467403de63317df4dfa72a1a0f241bdf5d2453ee2ec2c7f8e9c62451`
- `spec/assurance/MP-001-codegen-measurements.md` — blob `e5de9665e4f683bf5077c06e02e7e0ef59934a65`; SHA-256 `1793ce93b95793c0c75110c428c5f38d5cf1b1835ec72ac945bb303037025550`

#### `ix://agent-ix/quire-contract-ir`

- `spec/assurance/AP-001-contract-ir-v01.md` — blob `ad98de8dc90c538733c19a05de7d4953e8d196f6`; SHA-256 `00a562f4845a50966e446bd95a4d58b109e50984df4925f30b2fc730212ac92a`
- `spec/assurance/MP-001-contract-conformance.md` — blob `a251ecdb1dfc0850c76cd830d0f4a0a115f21bd2`; SHA-256 `60b65ae2d5506a9063e9991782db32d4a622ae58da3ad093a726a5e83ea6c068`

#### `ix://agent-ix/quire-contract-runtime`

- `spec/assurance/AP-001-runtime-release.md` — blob `11335af7bad40710c22f7465ddf4d59382a4b944`; SHA-256 `b433885d39de1aae2dfafe69f0846bb76bc082421ab37b6078eb90caa205ca27`
- `spec/assurance/MP-001-runtime-measurements.md` — blob `1bb9e0aad45d49429e3c85fa073e549e398c6a16`; SHA-256 `1b5d3b980ec12204abac6f0f34f5867f900c16e30955b1b88dfe90ee8710f1ca`

#### `ix://agent-ix/quire-rs`

- `spec/assurance/AP-201-detection-minting.md` — blob `ee1d52e55acf46a2f0c316c9a21bba761acf2f54`; SHA-256 `5aaf5756cd29c533e967a704c3a46ecf7ea948732ae26cfdc61ec6ce60e33c8b`
- `spec/assurance/MP-201-binding-read.md` — blob `e7b8a237bda3825214c39150d267dde6df401fa0`; SHA-256 `a02665b42a2e177c87e6e5fa9027eb15c6786df8476b2649005b62a0334eb123`
- `spec/assurance/MP-202-backed-trace.md` — blob `a5c1cc05a112ff049ff96486e0399ba3ecc0483e`; SHA-256 `9cb1c020eda86639fd9de4058b3235e7edf048623be0bcb7f2f8fafe3cafd189`
- `spec/assurance/MP-203-dead-tags.md` — blob `0f8cea760c8cdabedea2b29f7a070060195c2e33`; SHA-256 `610fb2e09737f116258ee2ab213a115e1f4b42d4d445c5adaa3781f8d07de952`
- `spec/assurance/MP-204-minting-repositories.md` — blob `cf24ff068ea0dabd4ca10f0b8267693c9f7787f4`; SHA-256 `6b4ce7b4a345c9f5ac8b42d2b5983c47c93f72aab092f4f6e52553cb83765112`
- `spec/assurance/MP-205-specific-properties.md` — blob `aeae30be10ce99873d98c1d5c23ec2583b9e7e80`; SHA-256 `1535e305ada2b870135923b98da2622075f974f67c0c5fca75d1807bbc88dfe8`
- `spec/assurance/MP-206-silent-zero.md` — blob `0f4e841b21525ba01722070c56820414f44ea2db`; SHA-256 `63badf3cadbe71701afec5250385ce8c512dfe41001a6887057a6b03184fad30`
- `spec/assurance/MP-207-skeptic-suspicion.md` — blob `a82fe3ab19aab68c09042b62ccee9714932fe438`; SHA-256 `eb67efb5c6a202cde3fa1cbc7ccc9ca711d03c9f508287ee645bf7ac90098aff`
- `spec/assurance/MP-208-authoring-tag-rate.md` — blob `9d713ba9007d2c9f631d05051029f8fc895526e4`; SHA-256 `64be8ed6120822dbadecf68f1c02983f047dffa3be194fa1dffc68302c9a0915`

#### `ix://agent-ix/quire-verification`

- `spec/assurance/AP-001-qualified-rust-path.md` — blob `a204c9300bd0a4c23e26d2e07682d55ebae9d572`; SHA-256 `6a49e8d6ff2b52462d996d1e3faed763e7d8dece2ae3d63ce3cd3fabe7765564`
- `spec/assurance/MP-001-parser-portfolio-effort.md` — blob `31beb4793f39cd71a09de413146323a5235bfd8f`; SHA-256 `9554a720b6efbad1859877dbe4ced4b8186ef90063d73442371adaec86cbd2aa`

#### `ix://agent-ix/quoin`

- `spec/assurance/AP-201-finding-quality.md` — blob `14324a31784b6f8a7094426e11b5b385e9ee0578`; SHA-256 `353de481ab24d972a305bbd91bda45fc1eec1c65f1136431697b134a252039f4`
- `spec/assurance/MP-201-finding-precision.md` — blob `d81adcfd23cb107a1449dd2cfd07b18450ac11a7`; SHA-256 `17a7ac0469805d10ee60d4d73ba42bd302fcc992309cdb38119125dd630bd85d`
- `spec/assurance/MP-202-finding-recall.md` — blob `7b36cdac03058501a30d7469deaabf6c57be07a3`; SHA-256 `50fd478a786cc5358c5bcd4d902802a46d0df4c0829c3b5fc347b68b8434505b`
- `spec/assurance/MP-203-unadjudicated-findings.md` — blob `de0e1047a9ecc189fa161a33392d7223bb8f4a7c`; SHA-256 `dc2f91d34d7bc8623214bfc8c32f2377f81dd530dc7147f22137d870338ff52b`
- `spec/assurance/MP-204-span-grounding.md` — blob `2d5ede1b16d3a17a9158d49cc16c6961e34cedf0`; SHA-256 `ee513ff3999291d6a1d7dedcaaa222e1e004209257aa51710049799f85e16282`
- `spec/assurance/MP-205-actionability.md` — blob `9fc2fbb3f484f82cdf65eb61d9484bc6d685b82c`; SHA-256 `c356239ced41237b7a643140d6468085deb2c534a8a8156b452d3f7b475451c8`
- `spec/assurance/MP-206-confirmed-insight-cost.md` — blob `e2ceac4a1f443e18f577510c0010bcf7e89a35a4`; SHA-256 `9f4ca18a6becbb41c8c2922cd3e947d2e522f0105620445b2bd5f4e1e32a1122`
- `spec/assurance/MP-207-silent-zero.md` — blob `718e28cdef6638adccb78e2ca33f472dc1cd0210`; SHA-256 `213d7c48fd2266572bdfdecaace539818bed6f52f121c17c10e634619c3e7fe5`
- `spec/assurance/MP-208-minting-section-hit.md` — blob `a2604ca3fbf4852c0d55e04238e5d40c7d0855bc`; SHA-256 `ca7bd1bb39bed0982a3c9bc725be33289d47c8c51cde107d8cafac8e34a92c0f`
- `spec/assurance/MP-209-finding-localisation.md` — blob `51f687c42fea69b7aee12b23211263bff491988f`; SHA-256 `19bb0156287cf937b59ff23fe4e0bfa3d6bfcb60b0b31aef8fa2a101878889c1`
- `spec/assurance/MP-210-detection-recall.md` — blob `51983d9b125113a8ca57f9e3059eeec5915470e1`; SHA-256 `789a6ecfd68ff4688258e1f0ee2436c3c6fc3f140dddeadc70c9a9007f453f0b`
- `spec/assurance/MP-211-corpus-gap-count.md` — blob `2541b5561ae4850d266361ba633e5556dc55fa0e`; SHA-256 `f29ac7f6dd6419ac039c1ca74504021ae4de5d0d10c33229b815f9a8a0271502`
- `spec/assurance/MP-212-actionability-v2.md` — blob `6ebb10bc1b30e0d2550a00c95e95e22457663f14`; SHA-256 `16adcfb173ac99645998b272bda3a7ab0470b1c1b253cfd7ba534390128c8732`
- `spec/assurance/MP-213-span-correctness.md` — blob `1ee9902497dcbb6c8999e0fb7badba2f0f6784b5`; SHA-256 `839576d9b49afe6fa9cb308ba172944f7755fcf23df96c17614837d38ab2a9bf`
- `spec/assurance/MP-214-safe-span-refusal.md` — blob `70ce5452cc3ec1c5b26c4dda06d45476890d34c2`; SHA-256 `e732a18ed36587d856f07811f8dc15c7b769c145561dd0439506a2aabdfa05f1`
- `spec/assurance/MP-215-span-grounding-v2.md` — blob `34c5a0dbe0fcaef0874f8e31406ddeacc7dec725`; SHA-256 `b6da433ea994da9ee1f24811506ac512691cffd2395970cc7c65a55f0afdc1b5`
- `spec/assurance/MP-216-guidance-correctness.md` — blob `3c9ccbf067272e9486d846cf48cfd9870f7144fa`; SHA-256 `b52fb8a7ab92a08962970886a9c5967eb93d3012e7fa0ea519a0d406b26c0098`
- `spec/assurance/MP-217-guidance-repair-success.md` — blob `9f341d49e9bf8bbd9107719989f2aae0207cf845`; SHA-256 `0bb882a28a4a8d82de73a2d69b61ec728eb046d66b93d631abe8320c41f4e2e2`
- `spec/assurance/MP-218-guidance-diagnostic-yield.md` — blob `3004c815e35bb70a2df4d70a993a992e63c01e5c`; SHA-256 `a0282551133d81789b42d5f2ec91b638a972608b3ca50d963c3715157f253476`
- `spec/assurance/MP-219-span-breadth.md` — blob `ac1ff34e19d663abc992f26891c798f8f947fe76`; SHA-256 `2214a09c4cdad100475f1ed2f368f16ff0c6953762bef94fec51d76f972374c2`
- `spec/assurance/MP-220-cli-eval-sentinel-contract.md` — blob `149a2f7db4e9ab3810879dea890c27d8a94d3208`; SHA-256 `8377b2050a2a76024e1b7f01a113e658dc59827698c7bdcffce0010a679ec1be`
- `spec/assurance/MP-221-quoin-release-operational-evidence.md` — blob `f263d15167e82e4199f13d4c27a35ab9e3787c3d`; SHA-256 `c35502861fadbf55872def11ef177f2a4f01fbc7db86ca00bc02550f56a77e82`

#### `ix://agent-ix/tl-mltl`

- `spec/assurance/AP-001.md` — blob `14a7f332fb6c8e8b442182cbd4b2adedeb8058d1`; SHA-256 `a96f0f72c6757caf8b966d6ae100e0f277a45d5ed06d84d02a8e1e1091c9e247`
- `spec/assurance/MP-001.md` — blob `eb2ad097fe3657ad9220af263de21f8a1a465fd8`; SHA-256 `4ca2e7a2e98d00e17e3379e05c85e2a2b746a35bf300232db2aef943fcb531f2`

#### `ix://agent-ix/tl-parse`

- `spec/assurance/AP-001.md` — blob `e1ab9fac8eee4e004671e3b5700e8d3939dac728`; SHA-256 `4e2d96ad78869e8aa84b648769df017845a0d930693a81bf60897e6c32b43625`
- `spec/assurance/MP-001.md` — blob `2140d6795e603d4384af5a1f110edd9932cc7f47`; SHA-256 `75ce98569b16e5b24b8bc45a552f12bcfa41189254a202ba1049c06f0b8af2dd`

#### `ix://agent-ix/tl-rewrite`

- `spec/assurance/AP-001.md` — blob `79ac169408d5954ac4ddac813cd81ce5a5f5dc77`; SHA-256 `0fd2e2f707f1c48d4c4dc665df1c5b4b7dd35a9c28d5f6b36324478702df4d49`
- `spec/assurance/MP-001.md` — blob `ff5f5e169d8fc9e566f70e93e4cf31cfcdd81a43`; SHA-256 `e98b3496b32a86b0eac0a66bfc9bd31ddda2ab096ea34c277a27fb3ae9ed5d83`

#### `ix://agent-ix/tl-syntax`

- `spec/assurance/AP-001.md` — blob `2ea3c1196c8d3f5ce9eb2f2f670c350c16f676f3`; SHA-256 `679a9b19a093768d1a8fc835c1129519a78e090b1738e63f0c51f8c1d55eb522`
- `spec/assurance/MP-001.md` — blob `8847b73981d07cff34a6e925ef6098e8641e2866`; SHA-256 `db421063e8fff65c7b6af1a7cb7f4d024491a6d24694d64a6cb8e96cbe9c01e2`

The separately bound installed provider candidate is module version `0.2.0`,
manifest SHA-256 `80e4f2c8...fc31f72`, AssuranceProfile schema SHA-256
`070f8802...6cf3535`, MeasurementPlan schema SHA-256
`aae266f2...2bcfd05a`, AssuranceProfile skeleton blob `e0188f0c...` and
SHA-256 `d7e007c8...1effd46`, and MeasurementPlan skeleton blob `d74500c0...`
and SHA-256 `2a4358d5...ba65a99`. The snapshot will bind the exact provider
candidate revision when it runs; the consumer registry cannot name its own
containing commit.

## Gitlink accounting

- Engineering Assurance pins `ix://agent-ix/qa-corpus@4b390c29...` at `corpus`.
- `quire-rs@8b8020e6...` pins `ix://agent-ix/qa-corpus@7442f277...`
  at `corpus`.
- `quoin@7072d65d...` pins `ix://agent-ix/qa-corpus@7b81343e...`
  at `corpus`.

These are references to the same canonical repository at different commits,
not three additional consumer repositories. TASK-017 must bind each gitlink
reference and deduplicate only identical `(repository, commit, artifact blob)`
contract inputs; it must not collapse different revisions.

## Criterion review

- **Complete:** the cardinality model can represent every observed artifact,
  the census covers all 294 non-archived organization repositories rather than
  a workstation directory or a 200-result client subset, and owner review has
  a fail-closed acceptance state.
- **Clear:** repository identity/commit and artifact path/classification are
  separate levels; duplicate rules apply at their respective levels.
- **Consistent:** the model preserves one population authority and treats
  package-corpus gitlinks as references, not independent repository identities.
- **Testable:** TC-122 now names one/many/empty cardinality cases as well as
  duplicate repository and duplicate repository-relative path cases.
- **Traceable and necessary:** both corrections directly unblock FR-015-AC-5
  through AC-7 and TASK-017 without expanding Engineering Assurance ownership.

## Review disposition

**CONDITIONAL.** FND-118 through FND-123 are closed by the specification changes and exact-byte evidence.
The proposed population and classifications still require explicit human-owner
acceptance, and TASK-017 implementation remains blocked on the independent
TASK-016 review. This review does not authorize #60 consumer migration.
