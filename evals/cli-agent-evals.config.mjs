// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

import { dirname } from "node:path";

const rootDir = dirname(import.meta.dirname);
const nativeProvider = process.env.ENGINEERING_ASSURANCE_BIN || "engineering-assurance";

export default {
  name: "engineering-assurance-onboarding",
  rootDir,
  provider: {
    command: nativeProvider,
    args: ["agent-evals-provider", "--root", rootDir],
  },
};
