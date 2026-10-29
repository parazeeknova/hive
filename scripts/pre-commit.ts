import { versionBump } from "bumpp";

import { root, run } from "./run";

const files = [
  "package.json",
  "apps/hiveway/package.json",
  "packages/local/package.json",
  "apps/nectar/package.json",
  "Cargo.toml",
  "lerna.json",
];
const generated = ["bun.lock", "Cargo.lock"];
// Never stage unrelated edits to files owned by the version hook.
await run(["git", "diff", "--exit-code", "--", ...files, ...generated]);
await run(["bun", "run", "check", "--ci"]);
const previous = Bun.spawn(["git", "show", "HEAD:package.json"], {
  cwd: root,
  stdout: "pipe",
});
const head: { version: string } = JSON.parse(
  await new Response(previous.stdout).text()
);
if ((await previous.exited) !== 0) {
  throw new Error("Cannot read the committed Hive version");
}
// Always derive from HEAD so a failed commit retry reuses the same patch.
await versionBump({
  commit: false,
  confirm: false,
  currentVersion: head.version,
  cwd: root,
  files,
  ignoreScripts: true,
  interface: false,
  push: false,
  release: "patch",
  tag: false,
});
await run(["bun", "install", "--lockfile-only"]);
await run(["cargo", "check", "--workspace", "--all-targets", "--all-features"]);
await run([
  "oxfmt",
  "--check",
  ...files.filter((file) => file.endsWith(".json")),
]);
await run(["git", "add", "--", ...files, ...generated]);
