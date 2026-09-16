# Contributing

Thanks for helping build Recall Gate. Setup lives in [docs/developing.md](docs/developing.md). This file covers how we work in the repo.

## Before you open a PR

1. Install the toolchain and dependencies per [docs/developing.md](docs/developing.md).
2. Run **`make check`** (format, clippy, tests). CI runs the same on Ubuntu.
3. Read the specs your change touches: [domain](docs/domain.md), [protocol](docs/protocol.md), [platforms](docs/platforms.md).

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

## Commits and review

- Use clear commit messages. Conventional prefixes (`feat:`, `fix:`, `chore:`, `docs:`) are welcome.
- Lock UI changes (`pr-wayland`, `pr-x11`) need screenshots or video per the plan review gate.

## Questions

Open a discussion or issue on GitHub if scope or product behavior is unclear. Domain illegal states in `docs/domain.md` are the source of truth for types.
