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
make check          # fmt, clippy, tests, conventional commits (matches CI)
make check-commits  # commit subjects only
cargo test -p recallgate-core
```

Commit messages must follow Conventional Commits. See [CONTRIBUTING.md](CONTRIBUTING.md#commit-messages).

Optional Nix: `nix develop`, then the same commands.

## Workspace layout (planned)

| Crate / path | Role |
| --- | --- |
| `crates/core` | Library; domain and persistence land here per plan. |
| `crates/daemon` | Daemon binary (not in tree until `pr-daemon`). |
| `crates/lock-wayland`, `crates/lock-x11` | Lock clients (later PRs). |
| `crates/mcp` | MCP adapter (later PR). |

Stack order is defined in [docs/plan.md](docs/plan.md). Stay inside one PR’s paths unless the plan stacks branches.

## Testing

- Default gate: **`make check`** before push.
- GUI or session-lock work: use nested compositors as described in [docs/plan.md](docs/plan.md) live lanes (nested Sway, Xephyr). Avoid locking the operator’s real session unless they ask.
- Merge-ready criteria for implementation PRs are in the plan (unit, live, perf), not only `cargo test`.

## Cloud Agent environment

No committed `.cursor/environment.json` yet. GitHub Actions runs on `ubuntu-24.04` with `make check`. GTK/Wayland system libraries matter once lock crates exist; see [docs/developing.md](docs/developing.md).

[docs/plan.md](docs/plan.md) may reference `pstack/` paths on `origin/main`. That tree is not in this repo unless added separately.

## When stuck

Read the spec doc for the layer you are changing. Do not extend scope beyond the active PR in [docs/plan.md](docs/plan.md). Open a question to the operator only for gaps the specs do not cover.
