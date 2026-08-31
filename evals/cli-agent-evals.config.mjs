import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { cpSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";

import {
  resultContract,
  validateResult,
  validateTerminalWorkflow,
} from "./result-contract.mjs";

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

function invokeIxFlow(command, runId, stateDir) {
  const completed = spawnSync(
    "ix-flow",
    [command, runId, "--state-dir", stateDir, "--json"],
    { encoding: "utf8" },
  );
  try {
    return {
      payload: JSON.parse(completed.stdout),
      raw: completed.stdout,
      exitCode: completed.status,
    };
  } catch {
    return {
      payload: { ok: false },
      raw: completed.stdout,
      exitCode: completed.status,
    };
  }
}

function terminalWorkflowEvidence(ctx, input, result) {
  const contract = input.result_contract.terminal_event_contract;
  if (!contract.required) return { failures: [], proof: null };
  const stateDirValue = input.input.workflow_state_dir;
  const runId = input.input.run_id;
  if (typeof stateDirValue !== "string" || typeof runId !== "string") {
    return {
      failures: ["terminal workflow binding missing"],
      proof: null,
    };
  }
  const stateDir = resolve(ctx.workDir, stateDirValue);
  const retainedPath = relative(ctx.workDir, stateDir);
  if (retainedPath.startsWith("..") || isAbsolute(retainedPath)) {
    return {
      failures: ["terminal workflow state path escapes workspace"],
      proof: null,
    };
  }

  const history = invokeIxFlow("history", runId, stateDir);
  const verification = invokeIxFlow("verify", runId, stateDir);
  const failures = validateTerminalWorkflow(
    result,
    input.result_contract,
    history.payload,
    verification.payload,
  );
  const terminalEvent = Array.isArray(history.payload?.data)
    ? history.payload.data.findLast(
        (event) =>
          event?.kind === "phase.advanced" &&
          event.payload?.from === "decision_ready",
      )
    : null;
  const acknowledgement = Array.isArray(history.payload?.data)
    ? history.payload.data.findLast(
        (event) => event?.kind === "gate.acknowledged",
      )
    : null;
  return {
    failures,
    proof: {
      run_id: runId,
      state_dir: retainedPath,
      history_digest: createHash("sha256").update(history.raw).digest("hex"),
      history_exit_code: history.exitCode,
      verification_exit_code: verification.exitCode,
      verification_ok: verification.payload?.ok === true,
      acknowledgement,
      terminal_event: terminalEvent,
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
      if (expectation.input.workflow_state_dir) {
        mkdirSync(join(ctx.workDir, expectation.input.workflow_state_dir), {
          recursive: true,
        });
      }
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
      "Follow result_contract.terminal_event_contract exactly: write null when required is false; when required is true, copy its fixed workflow, version, owner, choice, and outcome fields plus its exact run_id, then copy timestamp from the real ix-flow phase.advanced event whose payload.from is decision_ready and payload.to is the required terminal outcome; never use the gate.acknowledged timestamp and never invent either value.",
      "When EVALUATION_INPUT.json input names workflow_state_dir and run_id, use that exact run ID and pass --state-dir with that repository-relative directory to every ix-flow command; never use global ix-flow state.",
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
      const workflow = terminalWorkflowEvidence(ctx, input, result);
      failures.push(...workflow.failures);
      return {
        ok: failures.length === 0,
        checks: {
          evaluation_result: result,
          terminal_workflow: workflow.proof,
        },
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
