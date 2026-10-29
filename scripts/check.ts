import { checkThenFix, run } from "./run";

if ((await run(["lerna", "run", "check"], undefined, false)) !== 0) {
  await checkThenFix(
    ["oxlint", "--deny-warnings", "."],
    ["oxlint", "--fix", "."]
  );
  await checkThenFix(["oxfmt", "--check", "."], ["oxfmt", "--write", "."]);
  await checkThenFix(
    [
      "cargo",
      "clippy",
      "--locked",
      "--workspace",
      "--all-targets",
      "--all-features",
      "--",
      "-D",
      "warnings",
    ],
    [
      "cargo",
      "clippy",
      "--fix",
      "--allow-dirty",
      "--allow-staged",
      "--locked",
      "--workspace",
      "--all-targets",
      "--all-features",
    ]
  );
  await checkThenFix(
    ["cargo", "fmt", "--all", "--check"],
    ["cargo", "fmt", "--all"]
  );
  await run(["lerna", "run", "check"]);
}
