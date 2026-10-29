import { run } from "./run";

await run([
  "cargo",
  "test",
  "--locked",
  "--workspace",
  "--all-targets",
  "--all-features",
]);
await run(["bun", "run", "check-types", "--ci"]);
