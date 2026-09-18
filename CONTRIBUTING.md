# Contributing

Thanks for helping build Recall Gate. Setup lives in [docs/developing.md](docs/developing.md). This file covers how we work in the repo.

**Design center:** the desktop gate and local MCQ deck. Optional deck import (for example Anki) is a side path. Do not shape core types, stores, or RPC around Anki’s note or card model unless a spec PR says so. See [docs/domain.md](domain.md#what-drives-implementation).

## Before you open a PR

1. Install the toolchain and dependencies per [docs/developing.md](docs/developing.md).
2. Run **`make check`** (format, clippy, tests). CI runs the same on Ubuntu.
3. Read the specs your change touches: [domain](docs/domain.md), [protocol](docs/protocol.md), [platforms](docs/platforms.md).

### Tests in `crates/core`

Put behavioral tests in **`crates/core/tests/*.rs`** (integration tests against the public `recallgate_core` API). Keep `src/` free of `#[cfg(test)]` blocks unless a test must reach private items.

## PR scope

The implementation stack in [docs/plan.md](docs/plan.md) defines crate boundaries. Keep diffs inside one PR’s crate unless the plan explicitly stacks work.

| PR area | Paths |
| --- | --- |
| Core types | `crates/core/**`, workspace `Cargo.toml`, optional `flake.nix` |
| Store | `crates/core/**` (persistence) |
| Daemon | `crates/daemon/**`, `docs/protocol.md` when RPC changes |
| Wayland lock | `crates/lock-wayland/**` |
| X11 lock | `crates/lock-x11/**` |
| MCP | `crates/mcp/**` |

## Product rules (do not regress)

- There is **no** `gate_unlock` RPC or MCP tool.
- The **daemon** is the only SQLite writer. Lock binaries use the Unix socket only.
- Do not put AnkiConnect or HTML templates on the freeze path.
- Wayland lock uses **session lock**, not a layer-shell overlay as the product lock.

## Commit messages

Every commit must follow [Conventional Commits](https://www.conventionalcommits.org/).

```
type(optional-scope): short description
```

| Type | Use for |
| --- | --- |
| `feat` | New behavior |
| `fix` | Bug fix |
| `docs` | Documentation only |
| `style` | Formatting, no logic change |
| `refactor` | Code change without fixing or adding behavior |
| `perf` | Performance |
| `test` | Tests only |
| `build` | Build system or dependencies |
| `ci` | CI configuration |
| `chore` | Maintenance, tooling |
| `revert` | Reverts a prior commit |

Rules:

- Use lowercase for the type. Scope is optional (`feat(daemon): …`).
- Description is imperative and concise. No trailing period.
- Breaking changes: `type!:` or a `BREAKING CHANGE:` footer in the body.

CI rejects pull requests with non-conventional subjects. Locally, run **`make check-commits`** (also part of **`make check`**).

## Review

- Lock UI changes (`pr-wayland`, `pr-x11`) need screenshots or video per the plan review gate.

## Questions

Open a discussion or issue on GitHub if scope or product behavior is unclear. Domain illegal states in `docs/domain.md` are the source of truth for types.
