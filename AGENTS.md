# Agent guide

Operational notes for coding agents working in this repository. Product behavior and API rules live in `docs/` and [CONTRIBUTING.md](CONTRIBUTING.md), not here.

## Documentation map

| Doc | Use when |
| --- | --- |
| [docs/developing.md](docs/developing.md) | Toolchain, system packages, local commands, CI parity. |
| [docs/plan.md](docs/plan.md) | PR order, crate boundaries, verification expectations. |
| [CONTRIBUTING.md](CONTRIBUTING.md) | What belongs in a PR before you open it. |
| [docs/domain.md](docs/domain.md) | Domain types and naming. |
| [docs/protocol.md](docs/protocol.md) | Daemon RPC and MCP surface. |
| [docs/platforms.md](docs/platforms.md) | Platform backends and packaging. |

## Commands

```bash
make check          # fmt, clippy, tests, commit subjects (local pre-push)
make ci             # fmt, clippy, tests (same as Rust CI job)
make check-commits  # commit subjects only
cargo test -p recallgate-core
```

Commit messages must follow Conventional Commits. See [CONTRIBUTING.md](CONTRIBUTING.md#commit-messages).

Optional Nix: `nix develop`, then the same commands.

## Workspace layout

| Crate / path | Role |
| --- | --- |
| `crates/core` | Domain types and SQLite store |
| `crates/daemon` | JSON-RPC daemon (`recallgate-daemon`) |
| `crates/lock-wayland` | Wayland session-lock client (`--features ui`) |
| `crates/lock-x11` | X11 grab client (`--features x11`) |
| `crates/mcp` | MCP adapter (`recallgate-mcp`) |

Stack order in [docs/plan.md](docs/plan.md) is historical. Follow-up work that spans crates is allowed when the operator asks for one PR.

## Testing

- **`recallgate-core`:** integration tests live under `crates/core/tests/` (one file per area: `gate`, `item`, `ids`, `abort`). Do not add `#[cfg(test)]` modules in `src/` unless you need to exercise private internals; prefer the public API in `tests/`.
- Default gate: **`make check`** before push.
- GUI or session-lock work: use nested compositors as described in [docs/plan.md](docs/plan.md) live lanes (nested Sway, Xephyr). Avoid locking the operator’s real session unless they ask.
- Merge-ready criteria for implementation PRs are in the plan (unit, live, perf), not only `cargo test`.

## Cloud Agent environment

No committed `.cursor/environment.json` yet. GitHub Actions runs on `ubuntu-24.04` with `make check`. GTK/Wayland system libraries matter once lock crates exist; see [docs/developing.md](docs/developing.md).

[docs/plan.md](docs/plan.md) may reference `pstack/` paths on `origin/main`. That tree is not in this repo unless added separately.

## When stuck

Read the spec doc for the layer you are changing. Do not extend scope beyond the active PR in [docs/plan.md](docs/plan.md). Open a question to the operator only for gaps the specs do not cover.
