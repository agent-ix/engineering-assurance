import { cpSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { defineSuite } from "@agent-ix/cli-agent-evals";

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
    mkdirSync(join(ctx.workDir, "spec"), { recursive: true });
    writeFileSync(
      join(ctx.workDir, "EVALUATION_INPUT.json"),
      `${JSON.stringify({
        scenario: id,
        suite_revision: fixture.suite_revision,
        fixture_revision: fixture.fixture_revision,
        expectation,
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
    `Execute the ${id} scenario from EVALUATION_INPUT.json using real Quire, Quoin, and ix-flow boundaries where applicable.`,
    "Do not create an unsupported artifact, evidence claim, applicability decision, or terminal outcome.",
    "Write EVALUATION_RESULT.json with the immutable governing-version tuple, observed outcome, command and human-interaction counts, terminal event if any, and unsupported additions.",
  ].join(" "),
}));

export default defineSuite({
  name: "engineering-assurance-onboarding",
  rootDir,
  scenarios,
  assert(ctx, scenario, run) {
    const resultPath = join(ctx.workDir, "EVALUATION_RESULT.json");
    try {
      const result = JSON.parse(readFileSync(resultPath, "utf8"));
      const failures = [];
      if (run.exitReason !== "complete") failures.push(`agent exit: ${run.exitReason}`);
      if (result.observed_outcome !== scenario.expect.expected) {
        failures.push("observed outcome mismatch");
      }
      if (!Array.isArray(result.unsupported_additions) || result.unsupported_additions.length) {
        failures.push("unsupported additions present or unreported");
      }
      for (const field of [
        "governing",
        "command_count",
        "elapsed_ms",
        "human_prompt_count",
        "manual_translation_count",
        "repeated_prompt_count",
      ]) {
        if (result[field] === undefined || result[field] === null) {
          failures.push(`result field missing: ${field}`);
        }
      }
      if (scenario.expect.choice && result.terminal_event?.choice !== scenario.expect.choice) {
        failures.push("explicit terminal event missing");
      }
      return { ok: failures.length === 0, failures };
    } catch (error) {
      return { ok: false, failures: [`result envelope unreadable: ${error.message}`] };
    }
  },
});
