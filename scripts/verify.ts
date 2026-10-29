import path from "node:path";

import { root, run } from "./run";

const [app, task] = process.argv.slice(2);

if (app === "hiveway" && task === "check") {
  await run(["cargo", "fmt", "--all", "--check"]);
  await run([
    "cargo",
    "clippy",
    "--locked",
    "--workspace",
    "--all-targets",
    "--all-features",
    "--",
    "-D",
    "warnings",
  ]);
} else if (app === "hiveway" && task === "check-types") {
  // cargo check invokes rustc with the resolved dependency graph.
  await run([
    "cargo",
    "check",
    "--locked",
    "--workspace",
    "--all-targets",
    "--all-features",
  ]);
  await run(["cargo", "machete", "--with-metadata"]);
  await run(["cargo", "miri", "setup"]);
  await run([
    "cargo",
    "miri",
    "test",
    "--locked",
    "--workspace",
    "--all-features",
  ]);
} else if (app === "nectar" && task === "check") {
  await run(["oxlint", "--deny-warnings", "."]);
  await run(["oxfmt", "--check", "."]);
} else if (app === "nectar" && task === "check-types") {
  await run(["next", "typegen"], path.resolve(root, "apps/nectar"));
  await run(["tsc", "--project", "tsconfig.json", "--noEmit"]);
  await run(["tsc", "--project", "apps/nectar/tsconfig.json", "--noEmit"]);
} else {
  throw new Error("Expected: verify.ts <hiveway|nectar> <check|check-types>");
}
