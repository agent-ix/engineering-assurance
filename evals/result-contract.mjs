export const governingIdentities = [
  "module",
  "plugin",
  "skill",
  "workflow",
  "quire",
  "quoin",
  "ix_flow",
  "schema",
  "producer",
];

const countFields = [
  "command_count",
  "elapsed_ms",
  "human_prompt_count",
  "manual_translation_count",
  "repeated_prompt_count",
];

function isRecord(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function canonical(value) {
  if (Array.isArray(value)) return value.map(canonical);
  if (!isRecord(value)) return value;
  return Object.fromEntries(
    Object.keys(value)
      .sort()
      .map((key) => [key, canonical(value[key])]),
  );
}

export function resultContract(expectation, environment) {
  const terminalEventContract = expectation.choice
    ? {
        required: true,
        required_fields: [
          "run_id",
          "workflow",
          "workflow_version",
          "owner",
          "choice",
          "outcome",
          "timestamp",
        ],
        workflow: environment.governing.workflow.name,
        workflow_version: environment.governing.workflow.version,
        owner: expectation.input.decision_owner,
        choice: expectation.choice,
        outcome: expectation.expected,
        run_id: "copy from the real ix-flow terminal event",
        timestamp: "copy from the real ix-flow terminal event",
      }
    : { required: false, value: null };
  return {
    revision: "evaluation-result-v1",
    required_top_level_fields: [
      "host",
      "host_version",
      "source_revision",
      "suite_revision",
      "fixture_revision",
      "governing",
      ...countFields,
      "observed_outcome",
      "terminal_event",
      "unsupported_additions",
    ],
    host: environment.host,
    host_version: environment.host_version,
    source_revision: environment.source_revision,
    suite_revision: environment.suite_revision,
    fixture_revision: environment.fixture_revision,
    governing: environment.governing,
    governing_identities: governingIdentities,
    governing_identity_fields: ["name", "version", "digest"],
    governing_digest_format: "lowercase sha256",
    observed_outcome: expectation.expected,
    terminal_event_contract: terminalEventContract,
    unsupported_additions: [],
    count_type: "non-negative integer",
  };
}

export function validateResult(result, contract, exitReason) {
  const failures = [];
  if (exitReason !== "complete") failures.push(`agent exit: ${exitReason}`);
  if (!isRecord(result))
    return [...failures, "result envelope is not an object"];

  for (const field of [
    "host",
    "host_version",
    "source_revision",
    "suite_revision",
    "fixture_revision",
    "observed_outcome",
  ]) {
    if (result[field] !== contract[field]) {
      failures.push(`result field mismatch: ${field}`);
    }
  }
  if (
    JSON.stringify(canonical(result.governing)) !==
    JSON.stringify(canonical(contract.governing))
  ) {
    failures.push("governing versions mismatch");
  }
  if (
    !Array.isArray(result.unsupported_additions) ||
    result.unsupported_additions.length
  ) {
    failures.push("unsupported additions present or unreported");
  }
  for (const field of countFields) {
    if (!Number.isInteger(result[field]) || result[field] < 0) {
      failures.push(`result field invalid: ${field}`);
    }
  }

  const terminal = contract.terminal_event_contract;
  if (terminal.required) {
    if (!isRecord(result.terminal_event)) {
      failures.push("explicit terminal event missing");
    } else {
      const actualFields = Object.keys(result.terminal_event).sort();
      const requiredFields = [...terminal.required_fields].sort();
      if (JSON.stringify(actualFields) !== JSON.stringify(requiredFields)) {
        failures.push("terminal event fields mismatch");
      }
      for (const field of [
        "workflow",
        "workflow_version",
        "owner",
        "choice",
        "outcome",
      ]) {
        if (result.terminal_event[field] !== terminal[field]) {
          failures.push(`terminal event field mismatch: ${field}`);
        }
      }
      if (
        typeof result.terminal_event.run_id !== "string" ||
        !/^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/.test(result.terminal_event.run_id)
      ) {
        failures.push("terminal event run id invalid");
      }
      if (
        typeof result.terminal_event.timestamp !== "string" ||
        Number.isNaN(Date.parse(result.terminal_event.timestamp))
      ) {
        failures.push("terminal event timestamp invalid");
      }
    }
  } else if (result.terminal_event !== null) {
    failures.push("unexpected terminal event");
  }
  return failures;
}
