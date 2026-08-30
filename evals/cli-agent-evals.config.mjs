import { cpSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

import { resultContract, validateResult } from "./result-contract.mjs";

const rootDir = dirname(import.meta.dirname);
const fixture = JSON.parse(
  readFileSync(join(import.meta.dirname, "fixtures", "suite.json"), "utf8"),
);

function governingEnvironment(id) {
  const snapshotPath = process.env.EA_EVAL_GOVERNING_PATH;
  if (!snapshotPath) {
    throw new Error("EA_EVAL_GOVERNING_PATH is required for live evaluations");
  }
  const snapshot = JSON.parse(readFileSync(snapshotPath, "utf8"));
  const workflowName = [
    "interruption-resume",
    "human-acceptance",
    "human-rejection",
  ].includes(id)
    ? "architecture-evaluation"
    : "assurance-intake";
  return {
    host: snapshot.host.name,
    host_version: snapshot.host.version,
    source_revision: snapshot.source_revision,
    suite_revision: fixture.suite_revision,
    fixture_revision: fixture.fixture_revision,
    governing: {
      ...snapshot.governing,
      workflow: snapshot.workflows[workflowName],
    },
  };
}

const scenarios = Object.entries(fixture.scenarios).map(
  ([id, expectation]) => ({
    id: `EA-${String(Object.keys(fixture.scenarios).indexOf(id) + 1).padStart(3, "0")}`,
    useCase: id,
    canary: id === "existing-profile",
    expect: expectation,
    setup(ctx) {
      const skillSource = join(
        rootDir,
        "engineering_assurance",
        "skills",
        "assurance-onboarding",
      );
      for (const skillRoot of [
        ".agents/skills",
        ".claude/skills",
        ".github/skills",
      ]) {
        cpSync(
          skillSource,
          join(ctx.workDir, skillRoot, "assurance-onboarding"),
          {
            recursive: true,
          },
        );
      }
      mkdirSync(join(ctx.workDir, "spec"), { recursive: true });
      writeFileSync(
        join(ctx.workDir, "EVALUATION_INPUT.json"),
        `${JSON.stringify(
          {
            scenario: id,
            suite_revision: fixture.suite_revision,
            fixture_revision: fixture.fixture_revision,
            input: expectation.input,
            result_contract: resultContract(
              expectation,
              governingEnvironment(id),
            ),
          },
          null,
          2,
        )}\n`,
      );
      if (id === "existing-profile") {
        cpSync(
          join(
            rootDir,
            "engineering_assurance",
            "skeletons",
            "AssuranceProfile.md",
          ),
          join(ctx.workDir, "spec", "AP-001.md"),
        );
      }
      if (id === "malformed-producer") {
        mkdirSync(join(ctx.workDir, "producers"), { recursive: true });
        writeFileSync(
          join(ctx.workDir, "producers", "fictional.json"),
          "{invalid\n",
        );
      }
      if (id === "unavailable-producer") {
        mkdirSync(join(ctx.workDir, "producers"), { recursive: true });
        writeFileSync(
          join(ctx.workDir, "producers", "fictional.json"),
          `${JSON.stringify({ command: "fictional-missing-producer" }, null, 2)}\n`,
        );
      }
    },
    prompt: [
      "Use the installed assurance-onboarding skill for the fictional repository in this working directory.",
      `Execute the ${id} scenario from EVALUATION_INPUT.json using its explicit decision boundary and owner, and real Quire, Quoin, and ix-flow boundaries where applicable.`,
      "Do not create an unsupported artifact, evidence claim, applicability decision, or terminal outcome.",
      "Write EVALUATION_RESULT.json using EVALUATION_INPUT.json.result_contract exactly: copy its host, version, revision, and governing values verbatim; use the stated top-level field names and types; keep observed_outcome as the required string; and do not substitute aliases or structured objects for contract fields.",
      "Follow result_contract.terminal_event_contract exactly: write null when required is false; when required is true, copy its fixed workflow, version, owner, choice, and outcome fields, then copy run_id and timestamp from the real ix-flow terminal event without inventing either value.",
    ].join(" "),
  }),
);

export default {
  name: "engineering-assurance-onboarding",
  rootDir,
  scenarios,
  assert(ctx, scenario, run) {
    const resultPath = join(ctx.workDir, "EVALUATION_RESULT.json");
    try {
      const result = JSON.parse(readFileSync(resultPath, "utf8"));
      const input = JSON.parse(
        readFileSync(join(ctx.workDir, "EVALUATION_INPUT.json"), "utf8"),
      );
      const failures = validateResult(
        result,
        input.result_contract,
        run.exitReason,
      );
      return {
        ok: failures.length === 0,
        checks: { evaluation_result: result },
        failures,
      };
    } catch (error) {
      return {
        ok: false,
        failures: [`result envelope unreadable: ${error.message}`],
      };
    }
  },
};
