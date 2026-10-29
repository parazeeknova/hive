import { checkThenFix, run } from "./run";

await checkThenFix(
  ["sherif", "--fail-on-warnings"],
  ["sherif", "--fix", "--select", "highest", "--fail-on-warnings"]
);
// Advisory data changes independently of source files, so always audit outside Nx's cache.
await run(["cargo", "deny", "--locked", "check"]);
await run(["lerna", "run", "check-types"]);
