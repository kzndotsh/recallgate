# Recall Gate

Freeze the desktop until you answer one multiple-choice card.

Wrong answers still unlock after the correct choice flashes. Abort exists and costs more than answering. Agents can enqueue cards or start a freeze. They cannot unlock.

The workspace currently contains specs plus a minimal `recallgate-core` crate. Domain types and lock binaries follow [docs/plan.md](docs/plan.md).

## Developing

See [docs/developing.md](docs/developing.md) for toolchain setup, distro packages, and `make check`.

See [CONTRIBUTING.md](CONTRIBUTING.md) for PR scope and product rules.

Nix is optional: `nix develop` provides the same libraries as the documented apt/dnf/pacman lists.

## Docs

- [Agent guide](AGENTS.md) (for coding agents)
- [Developing](docs/developing.md)
- [Why it exists](docs/product.md)
- [Types and illegal states](docs/domain.md)
- [Daemon RPC and MCP](docs/protocol.md)
- [Wayland vs X11 vs later OS](docs/platforms.md)
- [PR stack and verification](docs/plan.md)
