#!/usr/bin/env node
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//
// One-time onboarding report for a repository that is about to use (or is
// already using) engineering-assurance. Run it once, at the start, before
// authoring an AssuranceProfile, MeasurementPlan, or measurement record.
//
// It answers the three questions that repeatedly cost people time:
//
//   1. How does this module (schema/skeleton SOURCE) relate to my project's
//      own `spec/assurance/` directory (instance HOME)?
//   2. Where are working examples of a valid measurement record, so I do not
//      have to read quoin-measurement's Rust source to learn the format?
//   3. What does a valid AssuranceProfile / MeasurementPlan / measurement
//      record actually require, before I author one and find out by
//      rejection?
//
// Usage:
//   node engineering_assurance/skills/assurance-onboarding/scripts/onboard.js [--repo <path>] [--json]
//
// `--repo` is the consuming project's root (default: current directory).
// `--json` emits the same report as machine-readable JSON instead of text.

import { readFileSync, existsSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const args = process.argv.slice(2);
const repoFlagIndex = args.indexOf("--repo");
const repoFlagValue = repoFlagIndex >= 0 ? args[repoFlagIndex + 1] : undefined;
if (repoFlagIndex >= 0 && (repoFlagValue === undefined || repoFlagValue.startsWith("--"))) {
  console.error("--repo requires a path argument");
  process.exit(1);
}
const targetRepo = path.resolve(repoFlagValue ?? process.cwd());
const wantsJson = args.includes("--json");

const scriptDir = path.dirname(fileURLToPath(import.meta.url));

// -- Locate this module's own schemas/, skeletons/, and manifest.yaml. -----
//
// The bundle root the four host plugin manifests resolve
// (`engineering_assurance/skills/assurance-onboarding`) always sits three
// levels below this script. A pip install target keeps schemas/ and
// skeletons/ as its direct siblings; some npm layouts hoist them to the
// package root instead. Try both before giving up on local resolution — the
// GitHub links below are printed unconditionally either way, so a failure
// here degrades the report rather than breaking it.
const candidateModuleRoots = [
  path.resolve(scriptDir, "..", "..", ".."), // .../engineering_assurance
  path.resolve(scriptDir, "..", "..", "..", ".."), // package root (npm hoist)
];
const moduleRoot = candidateModuleRoots.find(
  (candidate) => existsSync(path.join(candidate, "schemas")) && existsSync(path.join(candidate, "skeletons")),
);

// Read the manifest from the SAME candidate moduleRoot resolved above, not by
// re-scanning candidates independently — otherwise the reported version and
// the schemas actually loaded could come from two different install layouts.
let manifestVersion = null;
if (moduleRoot) {
  const manifestPath = path.join(moduleRoot, "manifest.yaml");
  if (existsSync(manifestPath)) {
    const match = readFileSync(manifestPath, "utf8").match(/^version:\s*(\S+)/m);
    if (match) manifestVersion = match[1];
  }
}
// Informational only: this module's own installed version. It does NOT pin
// the qa-corpus/quoin example links below — those are separate repositories
// with their own, unrelated release cadence, so they always point at `main`.
const installedModuleVersion = manifestVersion;

// -- Find, or fail to find, the dev-only corpus of worked examples. --------
//
// The corpus lives at the repository's top level, alongside (not inside)
// the installed `engineering_assurance/` module, and is deliberately not
// part of the published package (see package.json `files`). It is only on
// disk when this script runs from a full source checkout of
// agent-ix/engineering-assurance itself — which is exactly the case for
// anyone iterating on this module. Everyone else gets the GitHub link.
let corpusFiles = [];
if (moduleRoot) {
  const repoRoot = path.resolve(moduleRoot, "..");
  const corpusDir = path.join(repoRoot, "corpus", "spec", "evidence", "measurements");
  if (existsSync(corpusDir)) {
    corpusFiles = readdirSync(corpusDir)
      .filter((name) => name.endsWith(".json"))
      .sort()
      .map((name) => path.join(corpusDir, name));
  }
}

// -- Derive each artifact type's required frontmatter fields straight from --
// -- the schema files themselves, so this report can never drift from what --
// -- Quire actually enforces. Which schema file governs which type is read --
// -- from manifest.yaml's own `artifact_types` list, rather than a second, --
// -- hand-maintained copy of that mapping here. -----------------------------
const artifactSchemaFiles = {};
if (moduleRoot) {
  const manifestPath = path.join(moduleRoot, "manifest.yaml");
  if (existsSync(manifestPath)) {
    const manifestText = readFileSync(manifestPath, "utf8");
    // Each `artifact_types` entry starts with `- name: <Type>`; its
    // `frontmatter_schema_ref` follows before the next such entry. The same
    // `- name:` shape also opens each `grammars` entry, but those carry no
    // `frontmatter_schema_ref`, so they fall out of the schemaMatch filter
    // below rather than needing to be excluded up front.
    const nameMatches = [...manifestText.matchAll(/-\s*name:\s*(\S+)/g)];
    for (const [index, match] of nameMatches.entries()) {
      const end = nameMatches[index + 1]?.index ?? manifestText.length;
      const chunk = manifestText.slice(match.index, end);
      const schemaMatch = chunk.match(/frontmatter_schema_ref:\s*schemas\/(\S+)/);
      if (schemaMatch) artifactSchemaFiles[match[1]] = schemaMatch[1];
    }
  }
}

// A field required only in some cases (`allOf: [{ if, then: { required } }]`)
// is as rejecting as an unconditional one — a gate-stage MeasurementPlan
// without `ground_truth_kind` fails validation — so read those too. Without
// them this checklist would under-report exactly the requirement an author is
// most likely to miss.
//
// This only understands the one shape every schema in this module uses today:
// a flat `allOf` of `{ if: { properties, required }, then: { required } }`
// branches. A schema that instead used `else`, a nested `allOf`, `anyOf`,
// `oneOf`, `dependentRequired`, or a `$ref`'d branch would silently produce no
// output here rather than a wrong one — which is worse, because a silent gap
// looks like "no conditional requirements" instead of "this reader doesn't
// understand this schema". `readAllOf` below refuses to stay silent: anything
// it does not recognize becomes a `warnings` entry the report prints loudly.
const describeCondition = (condition, prefix = "") => {
  const clauses = Object.entries(condition?.properties ?? {}).map(([name, def]) => {
    const prop = `${prefix}${name}`;
    if ("const" in def) return `${prop} = ${def.const}`;
    if (def.enum) return `${prop} is one of ${def.enum.join(", ")}`;
    return `${prop} is present`;
  });
  return clauses.length > 0 ? clauses.join(" and ") : "(unrecognized condition)";
};

const readAllOf = (schema, prefix = "") => {
  const conditionalRequired = [];
  const warnings = [];
  if (schema.dependentRequired) {
    warnings.push("schema uses `dependentRequired`, which this checklist does not read");
  }
  if (schema.anyOf || schema.oneOf) {
    warnings.push("schema uses a top-level `anyOf`/`oneOf`, which this checklist does not read");
  }
  for (const branch of schema.allOf ?? []) {
    const conditionShape = branch.if?.$ref || branch.if?.anyOf || branch.if?.oneOf || branch.if?.allOf;
    if (conditionShape) {
      warnings.push(
        "an `allOf` branch's `if` uses $ref/anyOf/oneOf/allOf, which this checklist cannot describe",
      );
      continue;
    }
    if (branch.else) {
      warnings.push("an `allOf` branch has an `else`, which this checklist does not read");
    }
    if (branch.then?.allOf) {
      warnings.push("an `allOf` branch's `then` nests another `allOf`, which this checklist does not read");
    }
    if (Array.isArray(branch?.then?.required)) {
      conditionalRequired.push({
        when: describeCondition(branch.if, prefix),
        required: branch.then.required.map((name) => `${prefix}${name}`),
      });
    } else if (branch.then && Object.keys(branch.then).length > 0) {
      warnings.push(
        "an `allOf` branch's `then` has no `required` array this checklist reads, but does constrain something",
      );
    }
  }
  return { conditionalRequired, warnings };
};

// Resolve a same-document `#/$defs/<name>` reference, the only `$ref` shape
// these schemas use for a top-level field; anything else is read as written.
const resolveLocalRef = (schema, def) => {
  const ref = def?.$ref;
  if (typeof ref !== "string" || !ref.startsWith("#/$defs/")) return def;
  return schema.$defs?.[ref.slice("#/$defs/".length)] ?? def;
};

const artifactChecklists = {};
if (moduleRoot) {
  for (const [type, file] of Object.entries(artifactSchemaFiles)) {
    const schemaPath = path.join(moduleRoot, "schemas", file);
    if (!existsSync(schemaPath)) continue;
    let schema;
    try {
      schema = JSON.parse(readFileSync(schemaPath, "utf8"));
    } catch (error) {
      // Degrade the report rather than abort it: the GitHub links and the
      // other sections are still worth printing.
      artifactChecklists[type] = { schemaPath, unreadable: String(error) };
      continue;
    }
    const enums = {};
    const { conditionalRequired, warnings } = readAllOf(schema);
    for (const [prop, rawDef] of Object.entries(schema.properties ?? {})) {
      const def = resolveLocalRef(schema, rawDef);
      if (def.enum) enums[prop] = def.enum;
      // One level down: an object-valued field's own enums and conditional
      // requirements (a MeasurementPlan's `objective.direction`, and
      // `objective.bound` when that direction is `target`) reject a document
      // just as surely as top-level ones do.
      if (def.type === "object" && def.properties) {
        for (const [child, childDef] of Object.entries(def.properties)) {
          if (childDef.enum) enums[`${prop}.${child}`] = childDef.enum;
        }
        // The nested object's own `required` (e.g. `objective.direction`) is
        // not a top-level requirement — `objective` itself may be absent —
        // but it rejects a document just as surely once that object IS
        // present, so list it the same way as a conditional requirement.
        if (Array.isArray(def.required) && def.required.length > 0) {
          conditionalRequired.push({
            when: `${prop} is present`,
            required: def.required.map((child) => `${prop}.${child}`),
          });
        }
        const nested = readAllOf(def, `${prop}.`);
        conditionalRequired.push(...nested.conditionalRequired);
        warnings.push(...nested.warnings.map((warning) => `${prop}: ${warning}`));
      }
    }
    artifactChecklists[type] = {
      schemaPath,
      required: schema.required ?? [],
      conditionalRequired,
      enums,
      warnings,
    };
  }
}

// -- Inventory the target repo's own spec/assurance/ directory. ------------
//
// Read each file's own frontmatter `type:` field — the same field Quire and
// this module's native `onboarding` command key on — rather than guessing
// from the filename prefix. A renamed file or a hand-edited `type:` would
// otherwise report a type this report invented rather than the one on disk.
const specAssuranceDir = path.join(targetRepo, "spec", "assurance");
const frontmatterType = (filePath) => {
  let text;
  try {
    text = readFileSync(filePath, "utf8");
  } catch {
    return "unreadable";
  }
  if (!text.startsWith("---")) return "no-frontmatter";
  const end = text.indexOf("\n---", 3);
  const frontmatter = end >= 0 ? text.slice(0, end) : text;
  const match = frontmatter.match(/^type:\s*(\S+)/m);
  return match ? match[1] : "unrecognized";
};
let existingArtifacts = [];
const specAssuranceExists = existsSync(specAssuranceDir);
if (specAssuranceExists) {
  existingArtifacts = readdirSync(specAssuranceDir)
    .filter((name) => name.endsWith(".md"))
    .map((name) => ({ name, type: frontmatterType(path.join(specAssuranceDir, name)) }));
}

// -- The measurement-record checklist is NOT schema-file-governed like §4  --
// -- above: it is a hand-restated copy of quoin-measurement's Rust         --
// -- validator, with no mechanism here that would catch it drifting from   --
// -- that source. Every line below cites the file it was read from; if     --
// -- those crates change the contract, re-derive from the cited lines, not --
// -- from memory of this list. Verified against agent-ix/quoin at the      --
// -- revision current when PLAT-924 was authored:                         --
// --   rust/crates/quoin-measurement/src/validate/mod.rs                  --
// --   rust/crates/quoin-measurement/src/validate/stack.rs                --
// --   rust/crates/quoin-measurement/src/types/{collection,observation,ids}.rs --
// --   rust/crates/quoin-measurement/src/{discovery,store/publish}.rs     --
// --   rust/crates/quoin-store/src/digest.rs                              --
const measurementRecordChecklist = {
  collection: {
    // Checked on every accepted collection, historical (schemaVersion 1) or
    // current (2), whether it is merely being read or newly submitted.
    // validate/mod.rs `stored_measurement_collection`.
    always: [
      "schemaVersion — number, exactly 1 or 2 (mod.rs `schema_version()`)",
      "collectionId — non-empty string here; also the file name, so it separately must match ^[A-Za-z0-9._-]+$ at PUBLISH time, not at validate time (types/ids.rs `CollectionId::parse`, store/publish.rs)",
      "subject — non-empty string, what was measured",
      "toolIdentity — non-empty string",
      "toolVersion — non-empty string",
      "configDigest — non-empty string",
      "timestamp — non-empty string (no format is enforced beyond non-empty)",
      "sourceRevision — non-empty string (no format is enforced here — this is NOT the same rule as verificationStack.sources.*.revision below)",
      "scope — present (any JSON value; producer-defined)",
      "environment — a JSON object",
      "rawEvidence — present (any JSON value; complete producer output)",
      "observations — non-empty array (see below)",
      "corpusRevision — optional string",
    ],
    // Required, well-formed, whenever schemaVersion is 2 — for a STORED
    // record as much as a new one. validate/stack.rs `verification_stack()`.
    schemaVersion2VerificationStack: [
      "verificationStack — required object when schemaVersion is 2",
      "  schemaVersion — const \"verification-stack-attestation-v1\"",
      "  lockDigest, executableDigest — sha256:<64 LOWERCASE hex>; uppercase is refused (quoin-store digest.rs)",
      "  buildProfile — when present: \"debug\" or \"release\" (a stored record may omit it, or say \"debug\" — \"release\"-only is a NEW-collection-intake rule, below)",
      "  toolchains — when present: must name all of {node, rust, python}, each non-empty (a stored record may omit toolchains entirely — requiring it is a NEW-collection-intake rule, below)",
      "  sources — non-empty map; each entry: revision (exactly 40 LOWERCASE hex chars — a full, unabbreviated SHA, not the looser rule sourceRevision above gets), sourceState \"clean\", remote (non-empty)",
      "  capabilities — non-empty array of non-empty strings",
      "  artifacts — non-empty map of name -> sha256:<64 LOWERCASE hex> digest",
    ],
    // Enforced ONLY on the intake/publish path (`measurement_collection`,
    // used by write_measurement_collection when actually submitting a new
    // collection) — never by the read/stored path above. A collection
    // someone already reads back off disk is not held to these.
    newCollectionIntakeOnly: [
      "schemaVersion must be exactly 2 — a schemaVersion-1 collection is accepted for reading only, never for new submission",
      "verificationStack.buildProfile must be exactly \"release\"",
      "verificationStack.toolchains must be present and name all of {node, rust, python}",
      "each observation's metric must resolve to an ACTIVE MeasurementPlan discovered under spec/assurance/ OR assurance/ in the target repo (both are searched — discovery.rs), at the SAME definitionVersion, and observation.planId must equal that plan's id",
    ],
  },
  observation: [
    "metric — non-empty string",
    "planId — non-empty string (the MeasurementPlan id it believes governs it)",
    "definitionVersion — non-empty string",
    "unit — non-empty string",
    "shape — one of: scalar, ratio, count",
    "state — one of: measured, not_computed",
    "value — when state=measured: required numeric. When state=not_computed: the `value` KEY must be present and explicitly null — an ABSENT value key is refused, it is not treated as null (mod.rs `observation()`)",
    "no two observations in one collection may share the same (metric, sorted dimensions) pair — a duplicate is refused (mod.rs `stored_measurement_collection`)",
    "population — read only when it IS a JSON object: examined, matched, complete, identity (+ freeform extra fields, kept verbatim); a non-object population is silently treated as absent, not rejected",
    "dimensions — same silent-if-not-an-object treatment as population",
    "reason — free text; never required or format-checked by the validator, even when state=not_computed (a producer SHOULD still state one, but nothing will reject its absence)",
  ],
  sourceOfTruth:
    "agent-ix/quoin rust/crates/quoin-measurement: src/validate/mod.rs, src/validate/stack.rs, " +
    "src/types/{collection,observation,ids}.rs, src/discovery.rs, src/store/publish.rs, and " +
    "rust/crates/quoin-store/src/digest.rs — read this report instead of that source.",
};

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

const report = {
  targetRepo,
  moduleRoot,
  installedModuleVersion,
  relationship: {
    module:
      "engineering-assurance is the SCHEMA/SKELETON SOURCE: it supplies manifest.yaml, " +
      "schemas/*.schema.json (what Quire validates), and skeletons/*.md (starting templates). " +
      "It holds no project's actual decisions.",
    instances:
      `${path.join(targetRepo, "spec", "assurance")} is the INSTANCE HOME: every ` +
      "AssuranceProfile (AP-*.md), MeasurementPlan (MP-*.md), ArchitectureDescription (AD-*.md), " +
      "ComponentAssuranceContract (CAC-*.md), and AssuranceArgument (AA-*.md) this project actually " +
      "authors lives there, validated against the schemas the module supplies.",
  },
  specAssurance: {
    path: specAssuranceDir,
    exists: specAssuranceExists,
    existingArtifacts,
    note: specAssuranceExists
      ? null
      : "No spec/assurance/ yet — that is normal. Do not create one, or a generic " +
        "AssuranceProfile/MeasurementPlan inside it, until a stated decision boundary justifies it.",
  },
  workingExamples: {
    measurementCollections:
      corpusFiles.length > 0
        ? { source: "local checkout", files: corpusFiles }
        : {
            source:
              "GitHub (not present in this install — corpus/ is engineering-assurance's own " +
              "qa-corpus submodule, checked out only in a full dev clone with submodules initialized)",
            url: "https://github.com/agent-ix/qa-corpus/tree/main/spec/evidence/measurements",
          },
    realArtifactInstances: {
      note:
        "agent-ix/quoin's own spec/assurance/ is a live consumer of this module — real, validated " +
        "AssuranceProfile and MeasurementPlan instances, not just skeletons.",
      url: "https://github.com/agent-ix/quoin/tree/main/spec/assurance",
    },
    skeletons: moduleRoot
      ? readdirSync(path.join(moduleRoot, "skeletons")).map((f) => path.join(moduleRoot, "skeletons", f))
      : [],
  },
  artifactChecklists,
  measurementRecordChecklist,
  nextSteps: [
    "Decide the exact decision boundary and its human owner before authoring anything.",
    "Reuse an existing valid artifact in spec/assurance/ if one already covers the decision.",
    "Author from the module's skeleton, not from memory.",
    "Validate with `quire validate --scope . 'spec/assurance/**/*.md'` before treating it as real.",
    "Enter a workflow (assurance-intake / architecture-evaluation / measurement-promotion / " +
      "change-assurance) through ix-flow once the artifact is validated — see SKILL.md.",
  ],
};

if (wantsJson) {
  console.log(JSON.stringify(report, null, 2));
  process.exit(0);
}

const line = (text = "") => console.log(text);
const heading = (text) => {
  line();
  line(text);
  line("=".repeat(text.length));
};

heading("Engineering Assurance — onboarding report");
line(`Target repository: ${targetRepo}`);
line(`Module root:        ${moduleRoot ?? "(not found locally — using GitHub links only)"}`);

heading("1. How the two homes relate");
line(`- ${report.relationship.module}`);
line(`- ${report.relationship.instances}`);

heading("2. This repo's spec/assurance/");
if (specAssuranceExists) {
  line(`${specAssuranceDir} exists with ${existingArtifacts.length} artifact(s):`);
  for (const artifact of existingArtifacts) {
    line(`  - ${artifact.name} (${artifact.type})`);
  }
} else {
  line(report.specAssurance.note);
}

heading("3. Working examples — read these, not the Rust source");
if (report.workingExamples.measurementCollections.source === "local checkout") {
  line("Valid measurement-collection records on disk:");
  for (const file of corpusFiles) line(`  - ${file}`);
} else {
  line("Valid measurement-collection records (this install has no local corpus/):");
  line(`  - ${report.workingExamples.measurementCollections.url}`);
}
line();
line(report.workingExamples.realArtifactInstances.note);
line(`  - ${report.workingExamples.realArtifactInstances.url}`);
if (report.workingExamples.skeletons.length > 0) {
  line();
  line("Canonical starting skeletons in this module:");
  for (const file of report.workingExamples.skeletons) line(`  - ${file}`);
}

heading("4. What each artifact type actually requires");
if (Object.keys(artifactChecklists).length === 0) {
  line("(schemas not found locally — resolve your installed module root and re-run)");
} else {
  for (const [type, checklist] of Object.entries(artifactChecklists)) {
    line(`${type} (${checklist.schemaPath}):`);
    if (checklist.unreadable) {
      line(`  (schema could not be parsed: ${checklist.unreadable})`);
      line();
      continue;
    }
    line(`  required: ${checklist.required.join(", ")}`);
    for (const [prop, values] of Object.entries(checklist.enums)) {
      line(`  ${prop} must be one of: ${values.join(", ")}`);
    }
    for (const condition of checklist.conditionalRequired) {
      line(`  when ${condition.when}: also required: ${condition.required.join(", ")}`);
    }
    for (const warning of checklist.warnings ?? []) {
      line(`  WARNING: ${warning} — this checklist may be incomplete for ${type}`);
    }
    line();
  }
}

heading("5. What a measurement record actually requires");
line("(hand-restated from quoin-measurement's Rust validator, not schema-derived — see the source citation at the end of this section)");
line();
line("Every collection, read back or newly submitted (schemaVersion 1 or 2):");
for (const item of measurementRecordChecklist.collection.always) line(`  - ${item}`);
line();
line("Whenever schemaVersion is 2 — read back or newly submitted:");
for (const item of measurementRecordChecklist.collection.schemaVersion2VerificationStack) line(`  - ${item}`);
line();
line("A NEW collection being submitted (not a stored one being read) additionally requires:");
for (const item of measurementRecordChecklist.collection.newCollectionIntakeOnly) line(`  - ${item}`);
line();
line("Each observation:");
for (const item of measurementRecordChecklist.observation) line(`  - ${item}`);
line();
line(measurementRecordChecklist.sourceOfTruth);

heading("6. Next steps");
for (const step of report.nextSteps) line(`  - ${step}`);
line();
