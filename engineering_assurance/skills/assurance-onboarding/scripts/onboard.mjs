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
//   node engineering_assurance/skills/assurance-onboarding/scripts/onboard.mjs [--repo <path>] [--json]
//
// `--repo` is the consuming project's root (default: current directory).
// `--json` emits the same report as machine-readable JSON instead of text.

import { readFileSync, existsSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const args = process.argv.slice(2);
const repoFlagIndex = args.indexOf("--repo");
const targetRepo = path.resolve(
  repoFlagIndex >= 0 && args[repoFlagIndex + 1] ? args[repoFlagIndex + 1] : process.cwd(),
);
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

let manifestVersion = null;
for (const candidate of candidateModuleRoots) {
  const manifestPath = path.join(candidate, "manifest.yaml");
  if (existsSync(manifestPath)) {
    const match = readFileSync(manifestPath, "utf8").match(/^version:\s*(\S+)/m);
    if (match) {
      manifestVersion = match[1];
      break;
    }
  }
}
const releaseTag = manifestVersion ? `v${manifestVersion}` : "main";

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
// -- Quire actually enforces. ------------------------------------------------
const artifactSchemaFiles = {
  AssuranceProfile: "assurance-profile-frontmatter.schema.json",
  MeasurementPlan: "measurement-plan-frontmatter.schema.json",
  ArchitectureDescription: "architecture-description-frontmatter.schema.json",
  ComponentAssuranceContract: "component-assurance-contract-frontmatter.schema.json",
  AssuranceArgument: "assurance-argument-frontmatter.schema.json",
};

const artifactChecklists = {};
if (moduleRoot) {
  for (const [type, file] of Object.entries(artifactSchemaFiles)) {
    const schemaPath = path.join(moduleRoot, "schemas", file);
    if (!existsSync(schemaPath)) continue;
    const schema = JSON.parse(readFileSync(schemaPath, "utf8"));
    const enums = {};
    for (const [prop, def] of Object.entries(schema.properties ?? {})) {
      if (def.enum) enums[prop] = def.enum;
    }
    artifactChecklists[type] = {
      schemaPath,
      required: schema.required ?? [],
      enums,
    };
  }
}

// -- Inventory the target repo's own spec/assurance/ directory. ------------
const specAssuranceDir = path.join(targetRepo, "spec", "assurance");
const idPrefixes = {
  "AP-": "AssuranceProfile",
  "MP-": "MeasurementPlan",
  "AD-": "ArchitectureDescription",
  "CAC-": "ComponentAssuranceContract",
  "AA-": "AssuranceArgument",
};
let existingArtifacts = [];
const specAssuranceExists = existsSync(specAssuranceDir);
if (specAssuranceExists) {
  existingArtifacts = readdirSync(specAssuranceDir)
    .filter((name) => name.endsWith(".md"))
    .map((name) => {
      const prefix = Object.keys(idPrefixes).find((candidate) => name.startsWith(candidate));
      return { name, type: prefix ? idPrefixes[prefix] : "unrecognized" };
    });
}

// -- The measurement-record checklist is not schema-file-governed here: it --
// -- is enforced by quoin-measurement's Rust validator (validate/mod.rs,   --
// -- validate/stack.rs, types/collection.rs, types/observation.rs), and    --
// -- restated here so nobody has to read that source to learn the shape.   --
// -- Re-derive from that source if these crates change the contract.       --
const measurementRecordChecklist = {
  collection: {
    always: [
      "schemaVersion — 1 (historical, read-only) or 2 (current)",
      "collectionId — non-empty string; also the file name",
      "subject — non-empty string, what was measured",
      "toolIdentity — non-empty string",
      "toolVersion — non-empty string",
      "configDigest — non-empty string",
      "timestamp — non-empty string",
      "sourceRevision — non-empty string",
      "scope — present (any JSON value; producer-defined)",
      "environment — a JSON object",
      "rawEvidence — present (any JSON value; complete producer output)",
      "observations — non-empty array (see below)",
      "corpusRevision — optional string",
    ],
    schemaVersion2Additional: [
      "verificationStack — required object, schemaVersion \"verification-stack-attestation-v1\":",
      "  lockDigest, executableDigest — sha256:<64 hex>",
      "  buildProfile — must be \"release\" for a NEW collection",
      "  toolchains — {node, rust, python}, all non-empty, all required for a NEW collection",
      "  sources — non-empty map of {revision: <40-hex full SHA>, sourceState: \"clean\", remote}",
      "  capabilities — non-empty array of non-empty strings",
      "  artifacts — non-empty map of name -> sha256:<64 hex> digest",
      "each observation's metric must have an ACTIVE MeasurementPlan in spec/assurance/ at the same definitionVersion, and observation.planId must equal that plan's id",
    ],
  },
  observation: [
    "metric — non-empty string",
    "planId — non-empty string (the MeasurementPlan id it believes governs it)",
    "definitionVersion — non-empty string",
    "unit — non-empty string",
    "shape — one of: scalar, ratio, count",
    "state — one of: measured, not_computed",
    "value — required numeric when state=measured; must be null when state=not_computed",
    "population — optional object: examined, matched, complete, identity (+ freeform extra fields, kept verbatim)",
    "dimensions — optional object, freeform",
    "reason — optional string; state why when not_computed",
  ],
  sourceOfTruth:
    "agent-ix/quoin rust/crates/quoin-measurement: src/validate/mod.rs, src/validate/stack.rs, " +
    "src/types/collection.rs, src/types/observation.rs — read this report instead of that source.",
};

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

const report = {
  targetRepo,
  moduleRoot,
  releaseTag,
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
    line(`  required: ${checklist.required.join(", ")}`);
    for (const [prop, values] of Object.entries(checklist.enums)) {
      line(`  ${prop} must be one of: ${values.join(", ")}`);
    }
    line();
  }
}

heading("5. What a measurement record actually requires");
line("Every collection (schemaVersion 1 or 2):");
for (const item of measurementRecordChecklist.collection.always) line(`  - ${item}`);
line();
line("A NEW collection (schemaVersion 2) additionally requires:");
for (const item of measurementRecordChecklist.collection.schemaVersion2Additional) line(`  - ${item}`);
line();
line("Each observation:");
for (const item of measurementRecordChecklist.observation) line(`  - ${item}`);
line();
line(measurementRecordChecklist.sourceOfTruth);

heading("6. Next steps");
for (const step of report.nextSteps) line(`  - ${step}`);
line();
