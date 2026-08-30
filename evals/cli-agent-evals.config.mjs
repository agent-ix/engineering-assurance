import { cpSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

import { resultContract, validateResult } from "./result-contract.mjs";

const rootDir = dirname(import.meta.dirname);
const fixture = JSON.parse(
  readFileSync(join(import.meta.dirname, "fixtures", "suite.json"), "utf8"),
);

const scenarios = Object.entries(fixture.scenarios).map(([id, expectation]) => ({
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
      cpSync(skillSource, join(ctx.workDir, skillRoot, "assurance-onboarding"), {
        recursive: true,
      });
    }
    mkdirSync(join(ctx.workDir, "spec"), { recursive: true });
    writeFileSync(
      join(ctx.workDir, "EVALUATION_INPUT.json"),
      `${JSON.stringify({
        scenario: id,
        suite_revision: fixture.suite_revision,
        fixture_revision: fixture.fixture_revision,
        input: expectation.input,
        result_contract: resultContract(expectation),
      }, null, 2)}\n`,
    );
    if (id === "existing-profile") {
      cpSync(
        join(rootDir, "engineering_assurance", "skeletons", "AssuranceProfile.md"),
        join(ctx.workDir, "spec", "AP-001.md"),
      );
    }
    if (id === "malformed-producer") {
      mkdirSync(join(ctx.workDir, "producers"), { recursive: true });
      writeFileSync(join(ctx.workDir, "producers", "fictional.json"), "{invalid\n");
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
    "Write EVALUATION_RESULT.json using EVALUATION_INPUT.json.result_contract exactly: use the stated top-level field names and types, keep observed_outcome as the required string, and do not substitute aliases or structured objects for contract fields.",
  ].join(" "),
}));

export default {
  name: "engineering-assurance-onboarding",
  rootDir,
  scenarios,
  assert(ctx, scenario, run) {
    const resultPath = join(ctx.workDir, "EVALUATION_RESULT.json");
    try {
      const result = JSON.parse(readFileSync(resultPath, "utf8"));
      const failures = validateResult(result, scenario.expect, run.exitReason);
      return {
        ok: failures.length === 0,
        checks: { evaluation_result: result },
        failures,
      };
    } catch (error) {
      return { ok: false, failures: [`result envelope unreadable: ${error.message}`] };
    }
  },
};
