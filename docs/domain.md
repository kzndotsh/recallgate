# Domain reference

Names in this file are the only names for these ideas in `crates/core`. Product behavior lives in [product.md](product.md). Wire shapes live in [protocol.md](protocol.md).

## What drives implementation

Recall Gate is a **desktop gate** with a **local question deck**. The freeze loop, loan, bail, and MCQ on the lock surface are the center of the design.

Spaced repetition schedules when items become due. It is **not** an Anki clone. Optional **import** from external decks (for example Anki `.apkg`) may land later. Import copies rows into this domain. It does not run Anki at freeze time and does not shape v0 types.

Do not model notes, card types, HTML templates, or a second scheduler API in core because Anki has them.

## Identifiers (v0)

| Type | Meaning |
| --- | --- |
| `ItemId` | One schedulable MCQ unit (stem, choices, answer index, schedule) |
| `GateId` | One freeze episode (lock, optional loan, relock) |

v0 uses a single id per row. [protocol.md](protocol.md) may expose both `card_id` and `prompt_id` on the wire with the **same** `ItemId` until a future version needs separate presentation snapshots.

## Item (v0)

An `Item` has:

- `ItemId`
- `stem` string
- `choices` exactly four strings
- `correct_index` in `0..3`
- `schedule` (see below)
- `suspended` boolean. Suspended items never appear in due lists.

Optional `external_ref` string for tracing import source. Opaque to scheduling. Not required for local-only decks.

## Schedule (v0)

Scheduling is **pluggable**. Core stores a snapshot the active algorithm needs. v0 uses a small struct, not an Anki queue enum.

| Field | Role |
| --- | --- |
| `due_at` | When the item may be picked for a freeze |
| `reps`, `lapses` | Counters for analytics and scheduler input |
| `memory` | Algorithm-specific state (for example stability and difficulty if using FSRS defaults) |

There is no `New | Learning | Review | Relearning` enum in v0 core unless a PR adds it with a concrete scheduler requirement. Suspended is a flag on the item, not a queue.

## Rating and reviews

`Rating` is `Again`, `Hard`, `Good`, or `Easy`. There is no fifth grade.

`ReviewLog` is append-only. Each entry has `ItemId`, `Rating`, timestamps, and schedule snapshots before and after the rating. A review is produced only from answering the MCQ on the gate, not from bail.

v0 MCQ mapping: wrong index → `Again`, correct index → `Good`. Other grades are for later interaction kinds.

## Gate (freeze episode)

`GatePhase` is a sum type. Only these variants may be persisted as open state.

| Variant | Payload |
| --- | --- |
| `Idle` | None |
| `Locked` | `GateId`, `ItemId`, `started_at` |
| `Loan` | `GateId`, `until` |

`Unlocked` is a **transition outcome**, not a row stored as open state.

At most one `Locked` phase exists. Crash with `Locked` on disk reloads as `Locked` and the lock backend must freeze again.

When a loan ends, the daemon starts a new `Locked` phase (new `GateId`, pick due `ItemId`).

## Bail

`Bail` records method, timestamps, and cost paid. It is a gate event.

A bail path cannot create a `ReviewLog`. A review path cannot record a bail method.

## Due selection

`due_items()` returns `ItemId`s that are not suspended and have `due_at` in the past (or are new, per scheduler rules). The daemon picks one when `gate_lock` omits an item.

v0 source: local table of items. Agents may append rows via `gate_push_prompt`.

## Wire mapping (v0)

| Domain | JSON-RPC (see protocol) |
| --- | --- |
| `ItemId` | `card_id` and `prompt_id` (same value) |
| `GateId` | `session_id` |
| MCQ on lock | `stem` + `choices` (no `correct_index` in the lock payload) |

## Later (not v0 core)

These are explicitly out of scope until a dedicated PR defines them.

- Separate `PromptId` for immutable snapshots when item text changes but history must point at old wording.
- `TypedRecall`, reveal-then-grade, or HTML-bearing prompts.
- Anki `.apkg` import mapping into `Item` + `external_ref`.
- Dual sync with any external study app while the daemon is the writer.

## Illegal combinations

These must not be representable in v0.

- `Unlocked` stored as open gate state
- `Rating` without `ItemId` and `reviewed_at`
- Due list including a suspended item
- `Bail` that writes `ReviewLog`
- Two concurrent `Locked` phases
- Import or sync path that calls an external scheduler during a freeze
