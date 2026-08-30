const object = (value) =>
  value !== null && typeof value === "object" && !Array.isArray(value)
    ? value
    : null;

const values = (instance, kind) =>
  Array.isArray(instance.items?.[kind])
    ? instance.items[kind].map(object).filter(Boolean)
    : [];

const nonempty = (value) =>
  typeof value === "string" && value.trim() !== "";

const failure = (code, details = {}) => ({ ok: false, code, details });

const interviewItem = (instance, kind) =>
  values(instance, kind).find((item) => nonempty(item.interviewId)) ?? null;

const missing = (item, fields) =>
  fields.filter((field) => !nonempty(item?.[field]));

const observationReady = ({ instance }) => {
  const observations = values(instance, "operator_observation");
  const invalid = observations.filter(
    (item) =>
      !Number.isInteger(item.elapsed_minutes) ||
      item.elapsed_minutes < 0 ||
      !Number.isInteger(item.command_count) ||
      item.command_count < 1,
  );
  return observations.length > 0 && invalid.length === 0
    ? true
    : failure("operator_observation_missing_or_invalid");
};

const terminalKeys = {
  "assurance-intake": [
    "decision_ready->accepted",
    "decision_ready->rejected",
  ],
  "architecture-evaluation": [
    "decision_ready->accepted",
    "decision_ready->rejected",
  ],
  "measurement-promotion": [
    "decision_ready->promoted",
    "decision_ready->not_promoted",
  ],
  "change-assurance": [
    "decision_ready->approved",
    "decision_ready->rejected",
  ],
};

const terminalGates = ({ instance }) => {
  const configured = object(instance.gateConfig) ?? {};
  const changed = (terminalKeys[instance.defName] ?? []).filter(
    (key) => configured[key] !== "hitl",
  );
  return changed.length === 0
    ? true
    : failure("terminal_gate_override", { transitions: changed });
};

const exceptionsReady = ({ instance }) => {
  const request =
    interviewItem(instance, "intake_request") ??
    interviewItem(instance, "promotion_request") ??
    interviewItem(instance, "change_request");
  const exceptions = values(instance, "exception");
  const invalid = exceptions.filter(
    (item) =>
      missing(item, ["owner", "expires_at", "rationale", "impact"]).length > 0 ||
      !Number.isFinite(Date.parse(item.expires_at)) ||
      Date.parse(item.expires_at) <= Date.now(),
  );
  const required = request?.exceptions_expected === true;
  return invalid.length === 0 && (!required || exceptions.length > 0)
    ? true
    : failure("owned_current_exception_required");
};

const intakeScopeReady = ({ instance }) => {
  const request = interviewItem(instance, "intake_request");
  const fields = ["scope", "boundary", "impact_scenario", "owner"];
  return request && missing(request, fields).length === 0
    ? true
    : failure("intake_scope_incomplete");
};

const intakeArtifactsReady = ({ instance }) => {
  const request = interviewItem(instance, "intake_request");
  if (!request || !Array.isArray(request.requested_artifacts)) {
    return failure("requested_artifacts_missing");
  }
  const valid = new Set(
    values(instance, "artifact_validation")
      .filter((item) => item.valid === true)
      .map((item) => item.artifact_type),
  );
  const absent = request.requested_artifacts.filter((kind) => !valid.has(kind));
  return absent.length === 0
    ? true
    : failure("artifact_validation_missing", { artifact_types: absent });
};

const architectureScenariosReady = ({ instance }) => {
  const request = interviewItem(instance, "architecture_request");
  const description = values(instance, "artifact_validation").find(
    (item) =>
      item.valid === true &&
      item.artifact_type === "ArchitectureDescription" &&
      item.path === request?.description_path,
  );
  const required = ["concern", "stimulus", "environment", "response", "measure"];
  const scenarios = values(instance, "architecture_scenario");
  return description &&
    scenarios.length > 0 &&
    scenarios.every((item) => missing(item, required).length === 0)
    ? true
    : failure("architecture_context_incomplete");
};

const architectureReviewReady = ({ instance }) =>
  values(instance, "review_validation").some(
    (item) =>
      item.valid === true &&
      item.artifact_type === "SpecReview" &&
      item.analysis === "architecture-evaluation" &&
      item.subject_path ===
        interviewItem(instance, "architecture_request")?.description_path &&
      nonempty(item.path),
  )
    ? true
    : failure("architecture_review_missing");

const stages = [
  "observe",
  "baseline",
  "branch-comparison",
  "trend",
  "ratchet",
  "target",
  "gate",
];

const promotionReady = ({ instance }) => {
  const request = interviewItem(instance, "promotion_request");
  if (
    !request ||
    stages.indexOf(request.proposed_stage) !==
      stages.indexOf(request.prior_stage) + 1
  ) {
    return failure("promotion_must_advance_one_stage");
  }
  const required = [
    "plan_path",
    "definition_version",
    "prior_stage",
    "proposed_stage",
    "stability",
    "decision_yield",
    "limitations",
    "owner",
  ];
  const evidence = values(instance, "promotion_evidence").find(
    (item) =>
      item.plan_path === request.plan_path &&
      item.definition_version === request.definition_version &&
      item.prior_stage === request.prior_stage &&
      item.proposed_stage === request.proposed_stage,
  );
  return evidence && missing(evidence, required).length === 0
    ? true
    : failure("promotion_evidence_incomplete");
};

const changeImpactReady = ({ instance }) => {
  const request = interviewItem(instance, "change_request");
  const arrays = [
    "changed_nodes",
    "impacted_nodes",
    "missing_edges",
    "stale_evidence",
    "suspect_evidence",
    "unknowns",
  ];
  const snapshot = values(instance, "impact_snapshot").find(
    (item) =>
      item.source_revision === request?.source_revision &&
      item.profile_path === request?.profile_path &&
      item.baseline_id === request?.baseline_id,
  );
  const exactRevision = /^[0-9a-f]{40}$/.test(request?.source_revision ?? "");
  return exactRevision &&
    nonempty(request?.profile_path) &&
    nonempty(request?.baseline_id) &&
    snapshot &&
    arrays.every((field) => Array.isArray(snapshot[field]))
    ? true
    : failure("impact_snapshot_incomplete");
};

const digest = (value) =>
  typeof value === "string" && /^sha256:[0-9a-f]{64}$/.test(value);

const changeSnapshotReady = ({ instance }) => {
  const request = interviewItem(instance, "change_request");
  const snapshot = values(instance, "assurance_snapshot").find(
    (item) => item.source_revision === request?.source_revision,
  );
  const valid =
    snapshot &&
    snapshot.discharge_schema === "clause-discharge-v1" &&
    snapshot.argument_schema === "authored-assurance-view-v1" &&
    nonempty(snapshot.argument_id) &&
    digest(snapshot.discharge_digest) &&
    digest(snapshot.argument_digest) &&
    Number.isInteger(snapshot.binding_open) &&
    snapshot.binding_open >= 0 &&
    Number.isInteger(snapshot.applicability_unresolved) &&
    snapshot.applicability_unresolved >= 0 &&
    ["supported", "open", "challenged", "rejected"].includes(
      snapshot.top_claim_status,
    ) &&
    Array.isArray(snapshot.evidence_record_ids) &&
    snapshot.evidence_record_ids.length > 0 &&
    snapshot.evidence_record_ids.every(digest) &&
    Number.isFinite(Date.parse(snapshot.verified_at)) &&
    Date.parse(snapshot.verified_at) <= Date.now();
  return valid ? true : failure("assurance_snapshot_invalid");
};

const changeReviewReady = ({ instance }) =>
  values(instance, "review_validation").some(
    (item) =>
      item.valid === true &&
      item.artifact_type === "SpecReview" &&
      item.analysis === "code-review" &&
      item.source_revision ===
        interviewItem(instance, "change_request")?.source_revision &&
      nonempty(item.path),
  )
    ? true
    : failure("code_review_missing");

export const invariants = {
  "shared.observation_ready": observationReady,
  "shared.terminal_gates": terminalGates,
  "shared.exceptions_ready": exceptionsReady,
  "intake.scope_ready": intakeScopeReady,
  "intake.artifacts_ready": intakeArtifactsReady,
  "architecture.scenarios_ready": architectureScenariosReady,
  "architecture.review_ready": architectureReviewReady,
  "measurement.promotion_ready": promotionReady,
  "change.impact_ready": changeImpactReady,
  "change.snapshot_ready": changeSnapshotReady,
  "change.review_ready": changeReviewReady,
};
