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

export function resultContract(expectation) {
  return {
    revision: "evaluation-result-v1",
    required_top_level_fields: [
      "governing",
      ...countFields,
      "observed_outcome",
      "terminal_event",
      "unsupported_additions",
    ],
    governing_identities: governingIdentities,
    governing_identity_fields: ["name", "version", "digest"],
    governing_digest_format: "lowercase sha256",
    observed_outcome: expectation.expected,
    terminal_choice: expectation.choice ?? null,
    count_type: "non-negative integer",
  };
}

export function validateResult(result, expectation, exitReason) {
  const failures = [];
  if (exitReason !== "complete") failures.push(`agent exit: ${exitReason}`);
  if (!isRecord(result)) return [...failures, "result envelope is not an object"];

  if (result.observed_outcome !== expectation.expected) {
    failures.push("observed outcome mismatch");
  }
  if (!Array.isArray(result.unsupported_additions) || result.unsupported_additions.length) {
    failures.push("unsupported additions present or unreported");
  }
  for (const field of countFields) {
    if (!Number.isInteger(result[field]) || result[field] < 0) {
      failures.push(`result field invalid: ${field}`);
    }
  }

  if (!isRecord(result.governing)) {
    failures.push("governing versions missing");
  } else {
    for (const field of governingIdentities) {
      const identity = result.governing[field];
      if (
        !isRecord(identity) ||
        typeof identity.name !== "string" ||
        !identity.name.trim() ||
        typeof identity.version !== "string" ||
        !identity.version.trim() ||
        typeof identity.digest !== "string" ||
        !/^[0-9a-f]{64}$/.test(identity.digest)
      ) {
        failures.push(`governing identity invalid: ${field}`);
      }
    }
  }

  if (expectation.choice) {
    if (result.terminal_event?.choice !== expectation.choice) {
      failures.push("explicit terminal event missing");
    }
  } else if (result.terminal_event !== null) {
    failures.push("unexpected terminal event");
  }
  return failures;
}
