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

// A field required only in some cases (`allOf: [{ if, then }]`) is as
// rejecting as an unconditional one -- a gate-stage MeasurementPlan without
// `ground_truth_kind` fails validation -- so read those too. Without them this
// checklist would under-report exactly the requirement an author is most
// likely to miss.
//
// `readAllOf` understands these shapes, and only these, at a schema's own
// top level or at any depth inside a `$ref`'d (directly, or via an array's
// `items.$ref`) nested `$defs` entry (see `resolveLocalRef` and `visit`
// below):
// - an `allOf` branch `{ if, then, else? }` whose `if` is `properties` (at any
//   depth, each leaf a `const`, an `enum`, or bare presence) plus `required`,
//   and whose `then` is `required` and/or a describable constraint (see
//   `describeConstraint`);
// - draft-07 `dependencies` whose values are arrays of field names;
// - a non-empty `oneOf` whose every branch is exactly `{ required: [field] }`,
//   read as "exactly one of these fields".
// Anything else -- a `$ref`/`anyOf`/`oneOf`/`allOf` condition, a
// nested `allOf` in `then`, `dependentRequired`, an empty or other-shaped
// `oneOf`/`anyOf`, or a constraint keyword `describeConstraint` does not know
// -- becomes a `warnings` entry the report prints loudly. A silent gap would
// look like "no conditional requirements" instead of "this reader does not
// understand this schema".
const describeCondition = (condition, prefix = "") => {
  const clauses = Object.entries(condition?.properties ?? {}).map(([name, def]) => {
    const prop = `${prefix}${name}`;
    if ("const" in def) return `${prop} = ${def.const}`;
    if (def.enum) return `${prop} is one of ${def.enum.join(", ")}`;
    if (def.not && "const" in def.not) return `${prop} is not ${def.not.const}`;
    // A list condition (`contains`): some item of the list satisfies it, e.g.
    // a MeasurementPlan whose `negative_controls` has an `apparatus-edit` item.
    if (def.contains) return `${prop} has an item where ${describeCondition(def.contains)}`;
    if (def.properties) return describeCondition(def, `${prop}.`);
    return `${prop} is present`;
  });
  return clauses.length > 0 ? clauses.join(" and ") : "(unrecognized condition)";
};

// Describe what a `then` (or one `anyOf` branch inside it) demands of the
// fields under `prefix`, as clauses joined by "and". Returns null when the
// constraint uses a keyword this reader does not know, so the caller warns
// instead of printing a partial description.
const DESCRIBABLE_KEYWORDS = new Set(["$ref", "const", "enum", "not", "required", "properties", "anyOf", "type"]);
const describeConstraint = (def, prefix, path) => {
  if (def === false) return [`${path} must be absent`];
  if (typeof def !== "object" || def === null) return null;
  if (Object.keys(def).some((key) => !DESCRIBABLE_KEYWORDS.has(key))) return null;
  const clauses = [];
  if (def.$ref) {
    if (typeof def.$ref !== "string" || !def.$ref.startsWith("#/$defs/")) return null;
    clauses.push(`${path} must match ${def.$ref.slice("#/$defs/".length)}`);
  }
  if ("const" in def) clauses.push(`${path} must be ${def.const}`);
  if (def.enum) clauses.push(`${path} must be one of ${def.enum.join(", ")}`);
  if (def.not !== undefined) {
    if (Object.keys(def.not).length !== 1) return null;
    if ("const" in def.not) {
      clauses.push(`${path} must not be ${def.not.const}`);
    } else if (Array.isArray(def.not.required) && def.not.required.length > 0) {
      const fields = def.not.required.map((name) => `${prefix}${name}`);
      clauses.push(fields.length === 1
        ? `${fields[0]} must be absent`
        : `${fields.join(" and ")} must not appear together`);
    } else if (Array.isArray(def.not.anyOf) && def.not.anyOf.length > 0 &&
               def.not.anyOf.every((branch) => Object.keys(branch).length === 1 &&
                 Array.isArray(branch.required) && branch.required.length === 1)) {
      clauses.push(`${def.not.anyOf.map((branch) => `${prefix}${branch.required[0]}`).join(", ")} must be absent`);
    } else {
      return null;
    }
  }
  if (Array.isArray(def.required) && def.required.length > 0) {
    clauses.push(`${def.required.map((name) => `${prefix}${name}`).join(", ")} required`);
  }
  for (const [name, child] of Object.entries(def.properties ?? {})) {
    const nested = describeConstraint(child, `${prefix}${name}.`, `${prefix}${name}`);
    if (nested === null) return null;
    clauses.push(...nested);
  }
  if (def.anyOf) {
    if (def.anyOf.length === 0) return null;
    const branches = def.anyOf.map((branch) => describeConstraint(branch, prefix, path));
    if (branches.some((branch) => branch === null || branch.length === 0)) return null;
    clauses.push(`either ${branches.map((branch) => `(${branch.join(" and ")})`).join(" or ")}`);
  }
  return clauses;
};

const readAllOf = (schema, prefix = "", present = "") => {
  const conditionalRequired = [];
  const exactlyOneOf = [];
  const conditionalConstraints = [];
  const warnings = [];
  if (schema.dependentRequired) {
    warnings.push("schema uses `dependentRequired`, which this checklist does not read");
  }
  for (const [name, dependency] of Object.entries(schema.dependencies ?? {})) {
    if (Array.isArray(dependency)) {
      conditionalRequired.push({
        when: `${prefix}${name} is present`,
        required: dependency.map((required) => `${prefix}${required}`),
      });
    } else {
      warnings.push("schema uses a schema-valued `dependencies` entry, which this checklist does not read");
    }
  }
  const oneOfFields = (schema.oneOf ?? []).map((branch) =>
    Object.keys(branch).length === 1 && Array.isArray(branch.required) && branch.required.length === 1
      ? branch.required[0]
      : null,
  );
  const exactlyOneShape = oneOfFields.length > 0 && oneOfFields.every((field) => field !== null);
  if (exactlyOneShape) {
    exactlyOneOf.push({
      when: present || "always",
      fields: oneOfFields.map((field) => `${prefix}${field}`),
    });
  } else if (schema.oneOf) {
    warnings.push("schema uses a `oneOf` that is empty or not one `required` field per branch, which this checklist does not read");
  }
  if (schema.anyOf) {
    warnings.push("schema uses a top-level `anyOf`, which this checklist does not read");
  }
  if (schema.if || schema.then) {
    warnings.push(
      "schema uses a bare `if`/`then` not wrapped in `allOf`, which this checklist does not read",
    );
  }
  for (const branch of schema.allOf ?? []) {
    const conditionShape = branch.if?.$ref || branch.if?.anyOf || branch.if?.oneOf || branch.if?.allOf;
    if (conditionShape) {
      warnings.push(
        "an `allOf` branch's `if` uses $ref/anyOf/oneOf/allOf, which this checklist cannot describe",
      );
      continue;
    }
    if (branch.then?.allOf) {
      warnings.push("an `allOf` branch's `then` nests another `allOf`, which this checklist does not read");
      continue;
    }
    const when = describeCondition(branch.if, prefix);
    const { required, ...rest } = branch.then ?? {};
    if (Array.isArray(required) && required.length > 0) {
      conditionalRequired.push({ when, required: required.map((name) => `${prefix}${name}`) });
    }
    // Whatever the `then` demands beyond `required` -- a forbidden value, a
    // restricted enum, an `anyOf` of alternatives -- is described alongside,
    // so a `then` carrying both is reported in full.
    if (Object.keys(rest).length > 0) {
      const clauses = describeConstraint(rest, prefix, prefix.replace(/\.$/, ""));
      if (clauses === null || clauses.length === 0) {
        warnings.push("an `allOf` branch's `then` constrains something this checklist cannot describe");
      } else {
        conditionalConstraints.push({ when, constraint: clauses.join(" and ") });
      }
    }
    if (branch.else) {
      const clauses = describeConstraint(branch.else, prefix, prefix.replace(/\.$/, ""));
      if (clauses === null || clauses.length === 0) {
        warnings.push("an `allOf` branch's `else` constrains something this checklist cannot describe");
      } else {
        conditionalConstraints.push({ when: `otherwise (${when} does not hold)`, constraint: clauses.join(" and ") });
      }
    }
  }
  return { conditionalRequired, exactlyOneOf, conditionalConstraints, warnings };
};

// Resolve a same-document `#/$defs/<name>` reference: either a property's
// own `$ref` (an object embedded directly, e.g. MeasurementPlan's
// `objective` points at `$defs/objective`, whose own
// `required: ["direction"]` lives there, not on the schema's own top-level
// `required`), or an array property's `items.$ref` (e.g. AssuranceArgument's
// `reasoning` array points at `$defs/reasoning`). AssuranceArgument's
// `top_claim` similarly points at `$defs/claim`, whose own `evidence_refs`
// requirement is conditional on `status`, sitting inside that $defs entry's
// own `allOf`. Reading only `schema.required` and `schema.allOf` (as the
// top-level walk above does) would silently omit all of these. Anything
// else is read as written; a `$ref` found inside a `$defs` entry is left to
// the existing `$ref`-in-`if` warning above, not chased further.
const resolveLocalRef = (schema, def) => {
  const ref = def?.$ref ?? def?.items?.$ref;
  if (typeof ref !== "string" || !ref.startsWith("#/$defs/")) return def;
  return schema.$defs?.[ref.slice("#/$defs/".length)] ?? def;
};

// When a field has a retired-only legacy shape, its unconditional property
// remains broad. Read the non-retired branch's ref for the current authoring
// checklist; the legacy branch is separately described by readAllOf above.
const resolveCurrentConditionalRef = (schema, name, def) => {
  if (def?.properties || def?.$ref) return def;
  const ref = (schema.allOf ?? [])
    .map((branch) => branch.else?.properties?.[name]?.$ref)
    .find((value) => typeof value === "string" && value.startsWith("#/$defs/"));
  return ref ? schema.$defs?.[ref.slice("#/$defs/".length)] ?? def : def;
};

// The list keywords `visit` below describes; any other keyword on an array
// field becomes a warning.
const ARRAY_KEYWORDS = new Set(["type", "items", "minItems", "uniqueItems", "description"]);

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
    const arrays = {};
    const { conditionalRequired, exactlyOneOf, conditionalConstraints, warnings } = readAllOf(schema);
    // Walk object-valued fields (directly, or via a `$ref`/array
    // `items.$ref` resolved above) at every depth: a nested field's enums
    // and conditional requirements (a MeasurementPlan's
    // `objective.direction`, `objective.bound` when that direction is
    // `target`, `statistical_design.decision_rule.comparator`, or an
    // AssuranceArgument `top_claim.evidence_refs` when `top_claim.status` is
    // `supported`) reject a document just as surely as top-level ones do.
    // `seen` stops a self-referencing `$ref`.
    const visit = (properties, prefix, seen) => {
      for (const [prop, rawDef] of Object.entries(properties ?? {})) {
        const def = resolveLocalRef(schema, resolveCurrentConditionalRef(schema, prop, rawDef));
        const path = `${prefix}${prop}`;
        // A list field's own shape (a MeasurementPlan's `protected_apparatus`
        // or `negative_controls`): how many items, whether repeats are
        // refused, and what a string item must match -- its schema
        // `description` for the reader, the raw `pattern` in JSON only. A list
        // with no minimum, no uniqueness and no item pattern says nothing and
        // is left out. A list keyword this reader does not know is a warning,
        // not a silent omission.
        if (rawDef?.type === "array") {
          const unknown = Object.keys(rawDef).filter((key) => !ARRAY_KEYWORDS.has(key));
          if (unknown.length > 0) {
            warnings.push(`${path}: list uses ${unknown.join(", ")}, which this checklist does not read`);
          }
          // `def` is already the resolved `items.$ref` target when there is one.
          const itemDef = rawDef.items?.$ref ? def : rawDef.items;
          const list = {
            minItems: rawDef.minItems ?? 0,
            uniqueItems: rawDef.uniqueItems === true,
            ...(itemDef?.pattern ? { itemPattern: itemDef.pattern } : {}),
            ...(itemDef?.pattern && itemDef.description ? { itemDescription: itemDef.description } : {}),
          };
          if (list.minItems > 0 || list.uniqueItems || list.itemPattern) arrays[path] = list;
          // A list of closed values (an AssuranceProfile's
          // `measurement_policy.stages` or `review_policy.operations`): each
          // item must be one of them.
          if (!rawDef.items?.$ref && Array.isArray(itemDef?.enum)) enums[path] = itemDef.enum;
        }
        if (def.enum) enums[path] = def.enum;
        if (def.type !== "object" || !def.properties || seen.has(def)) continue;
        // The nested object's own `required` (e.g. `objective.direction`) is
        // not a top-level requirement -- `objective` itself may be absent --
        // but it rejects a document just as surely once that object IS
        // present, so list it the same way as a conditional requirement.
        if (Array.isArray(def.required) && def.required.length > 0) {
          conditionalRequired.push({
            when: `${path} is present`,
            required: def.required.map((child) => `${path}.${child}`),
          });
        }
        const nested = readAllOf(def, `${path}.`, `${path} is present`);
        conditionalRequired.push(...nested.conditionalRequired);
        exactlyOneOf.push(...nested.exactlyOneOf);
        conditionalConstraints.push(...nested.conditionalConstraints);
        warnings.push(...nested.warnings.map((warning) => `${path}: ${warning}`));
        visit(def.properties, `${path}.`, new Set([...seen, def]));
      }
    };
    visit(schema.properties, "", new Set());
    artifactChecklists[type] = {
      schemaPath,
      required: schema.required ?? [],
      conditionalRequired,
      exactlyOneOf,
      conditionalConstraints,
      enums,
      arrays,
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
      "configDigest — non-empty string (when schemaVersion is 2, also sha256:<64 LOWERCASE hex>, below)",
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
      "configDigest — sha256:<64 LOWERCASE hex> when schemaVersion is 2 (quoin >= 0.24.0; stack.rs `config_digest_shape`)",
      "verificationStack — required object when schemaVersion is 2",
      "  schemaVersion — const \"verification-stack-attestation-v1\"",
      "  lockDigest, executableDigest — sha256:<64 LOWERCASE hex>; uppercase is refused (quoin-store digest.rs)",
      "  buildProfile — when present: \"debug\" or \"release\" (a stored record may omit it, or say \"debug\" — \"release\"-only is a NEW-collection-intake rule, below)",
      "  toolchains — when present: node, rust and python are each a non-empty string, null, or absent, and at least one is set (quoin >= 0.24.0; quoin 0.23.1 requires all three, and cannot read a record that sets fewer). A stored record may omit toolchains entirely — requiring it is a NEW-collection-intake rule, below",
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
      "verificationStack.toolchains must be present (its per-language rule is above)",
      "each observation's metric must resolve to an ACTIVE MeasurementPlan discovered under spec/assurance/ OR assurance/ in the target repo (both are searched — discovery.rs; a file counts only when its frontmatter says type: MeasurementPlan), at the SAME definitionVersion, and observation.planId must equal that plan's id. Plans are keyed by metric; when two share one, the higher id wins whatever its status, so keep one plan per metric",
      "quoin >= 0.24.0, measured observations only (validate/population.rs): plan minimum_population requires population.examined >= it (QM-POPULATION-UNSTATED if absent, QM-POPULATION-BELOW-MINIMUM if smaller); plan repetitions above 1 requires population.repetitions >= it (QM-POPULATION-UNSTATED if absent, QM-REPETITIONS-SHORT if smaller)",
      "quoin >= 0.24.0: an artifacts entry whose name is a file in the repository must carry that file's digest; `quoin measurement record --digest-from-file artifacts.<name>=<path>` fills or checks it",
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
    for (const [prop, list] of Object.entries(checklist.arrays ?? {})) {
      const clauses = list.minItems > 0 ? [`at least ${list.minItems} item(s)`] : [];
      if (list.uniqueItems) clauses.push("no repeated item");
      if (list.itemPattern) {
        clauses.push(`each item: ${list.itemDescription ?? `matching ${list.itemPattern}`}`);
      }
      line(`  ${prop} is a list: ${clauses.join(", ")}`);
    }
    for (const condition of checklist.conditionalRequired) {
      line(`  when ${condition.when}: also required: ${condition.required.join(", ")}`);
    }
    for (const choice of checklist.exactlyOneOf ?? []) {
      line(`  when ${choice.when}: exactly one of: ${choice.fields.join(", ")}`);
    }
    for (const condition of checklist.conditionalConstraints ?? []) {
      line(`  when ${condition.when}: ${condition.constraint}`);
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
