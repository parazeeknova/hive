# CI

The `Hiveway` workflow runs five validation jobs in parallel:

| Job | Checks |
| --- | --- |
| `javascript` | Repository lint/format, workspace consistency, TypeScript, contract tests, Nectar production build |
| `rust-check` | Rust formatting, Clippy/type checking, fresh dependency audit, unused dependencies |
| `miri` | All isolated Miri tests |
| `local-platforms` | Native Hive Local and pinned upstream MCP tests on Linux, macOS and Windows |
| `integration` | All native Rust tests including PostgreSQL/security, payment fixtures, creator/buyer and MCP acceptance |

`validate` remains the aggregate required check. It fails if any validation job fails, is cancelled, or is skipped. Both container publications require `validate`, and deployment requires both published images. Superseded pull-request runs are cancelled. Pushes keep the active release running and only the newest pending commit, so stale queued releases cannot build a backlog. This uses GitHub's default [single pending slot](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency).

JavaScript runs directly on Node/Bun. Rust jobs use the development environment from `devenv.lock`; the CLI is installed from its pinned nixpkgs revision too. PostgreSQL and Redis run only in the integration job. Native tests and acceptance share a runner so they reuse compilation and infrastructure. The acceptance backend receives the read-only workflow token as `HIVE_GITHUB_TOKEN` to avoid the shared runner’s anonymous GitHub API quota. Repository analysis still requires explicit public visibility before fetching authenticated repository contents. The demo uses the pull request head revision.

Caches preserve Bun downloads, Next.js compilation, Cargo downloads, compiled Rust artifacts, the Miri sysroot, and the Nix store. Rust build caches are separated by job and toolchain. Each successful commit updates its build cache, with fallback keys allowing dependency reuse after source or lockfile changes. Debug information and incremental compilation are disabled only in CI to keep build caches smaller. Only Miri uploads the shared Nix environment cache. Docker publishing retains its existing per-image BuildKit caches.

The first run populates caches; compare later runs to measure the warm-cache benefit. Tests and security audits always execute even when build caches hit. For a suspected stale cache, bump the corresponding `v1` namespace in the setup action or workflow. Cache restore failures do not replace the normal build.

Validate workflow changes with `actionlint` and the repository's formatting check. Older actionlint releases may need to ignore the existing `ubuntu-26.04` runner label until their schemas catch up.
