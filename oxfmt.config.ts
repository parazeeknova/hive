import { defineConfig } from "oxfmt";
import ultracite from "ultracite/oxfmt";

export default defineConfig({
  ...ultracite,
  ignorePatterns: [
    ...(ultracite.ignorePatterns ?? []),
    "**/.next/**",
    "**/next-env.d.ts",
    "target/**",
    ".devenv/**",
    ".direnv/**",
    ".nx/**",
    "bun.lock",
    "devenv.lock",
  ],
});
