import { cpSync, existsSync, rmSync } from "node:fs";
import { join } from "node:path";

const root = process.cwd();
const source = join(root, "engineering_assurance");
const staged = ["manifest.yaml", "schemas", "skeletons"];

if (process.argv.includes("--clean")) {
  for (const name of staged) {
    rmSync(join(root, name), { recursive: true, force: true });
  }
  process.exit(0);
}

for (const name of staged) {
  const from = join(source, name);
  if (!existsSync(from)) {
    throw new Error(`missing package input: ${name}`);
  }
  cpSync(from, join(root, name), { recursive: true, errorOnExist: true });
}
