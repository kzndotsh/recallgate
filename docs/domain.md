# Domain reference

Names in this file are the only names for these ideas in `crates/core`. Product behavior lives in [product.md](product.md). Wire shapes live in [protocol.md](protocol.md).

## What drives implementation

Recall Gate is a **desktop gate** with a **local question deck**. The freeze loop, cooldown, abort, and MCQ on the lock surface are the center of the design.

v0 does **not** implement spaced repetition per card. You configure **how often the gate may fire**. Items are questions in a deck; when the gate fires, the daemon picks one (for example at random among non-suspended items).

Optional import from external decks and smarter scheduling may land later. They do not shape v0 types.

## Identifiers (v0)

| Type | Meaning |
| --- | --- |
| `ItemId` | One MCQ in the deck |
| `GateId` | One freeze episode (lock, optional cooldown, relock) |

v0 uses a single id per row. [protocol.md](protocol.md) may expose both `card_id` and `prompt_id` on the wire with the **same** `ItemId` until a future version needs separate presentation snapshots.

## Item (v0)

An `Item` has:

- `ItemId`
- `stem` string
- `choices` exactly four strings
- `correct_index` in `0..3`
- `suspended` boolean. Suspended items are not picked for a freeze.

Optional `external_ref` string for tracing import source. Not required for local-only decks.

There is no per-item `due_at`, interval, or scheduler state in v0.

## Gate cadence (v0)

**`GateCadence`** is user configuration for how often freezes are allowed to start.

| Field | Role |
| --- | --- |
| `lock_interval` | Minimum duration after an unlock before another freeze may start |

The compositor or idle helper (for example `swayidle`) may still decide *when* to request a lock. The daemon enforces that requests respect `lock_interval` since the last unlock, except for explicit manual or RPC `gate_lock` if the product allows those to bypass the interval.

Store `lock_interval` with daemon settings. It is not a field on each `Item`.

**Cooldown** after abort is separate: a fixed short unlock window, then relock. It is not the same setting as `lock_interval`.

## Answers (v0)

When the user answers the MCQ on the lock screen, core records an **`Answer`** (append-only): `ItemId`, chosen index, whether it was correct, and timestamp.

Wrong index → incorrect (product maps this to an `Again`-style outcome in the UI). Correct index → correct (`Good`-style). There is no per-card schedule update in v0.

An abort path cannot create an `Answer`. An answer path cannot record an abort.

## Gate (freeze episode)

`GatePhase` is a sum type. Only these variants may be persisted as open state.

| Variant | Payload |
| --- | --- |
| `Idle` | None |
| `Locked` | `GateId`, `ItemId`, `started_at` |
| `Cooldown` | `GateId`, `until` |

`Unlocked` is a **transition outcome**, not a row stored as open state.

At most one `Locked` phase exists. Crash with `Locked` on disk reloads as `Locked` and the lock backend must freeze again.

When a cooldown ends, the daemon starts a new `Locked` phase (new `GateId`, pick an `ItemId`).

## Abort

`Abort` records how the user escaped (chord, hold, confirm, and similar), timestamps, and cost paid. It is a gate event.

## Picking an item

`deck_items()` returns non-suspended `ItemId`s. `pick_item()` chooses one for the next lock (v0: uniform random unless a later PR defines round-robin).

`gate_due` on the wire lists items that may be shown (v0: same as active deck items). It does not mean each row has its own due time.

Agents append MCQs via `gate_push_prompt`.

## Wire mapping (v0)

| Domain | JSON-RPC (see protocol) |
| --- | --- |
| `ItemId` | `card_id` and `prompt_id` (same value) |
| `GateId` | `session_id` |
| MCQ on lock | `stem` + `choices` (no `correct_index` in the lock payload) |

## Later (not v0 core)

- Per-item scheduling (`due_at`, intervals, FSRS or other algorithms).
- `Rating` grades beyond correct / incorrect logging.
- Separate `PromptId` for immutable snapshots.
- Typed or HTML prompts.
- Deck import mapping.

## Illegal combinations

These must not be representable in v0.

- `Unlocked` stored as open gate state
- `Answer` without `ItemId` and timestamp
- Picking a suspended item for `Locked`
- `Abort` that writes an `Answer`
- Two concurrent `Locked` phases
