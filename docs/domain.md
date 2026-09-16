# Domain reference

Names in this file are the only names for these ideas. Implementers match these symbols later in `crates/core`.

## Identifiers

| Type | Meaning |
| --- | --- |
| `CardId` | Scheduled memory unit |
| `PromptId` | One presentation bound to a card |
| `SessionId` | One freeze |
| `NoteId` | Optional import identity. Never scheduled |

`CardId` and `PromptId` are not interchangeable.

## Prompt

A `Prompt` has `PromptId`, `CardId`, `stem`, and `ResponseKind`.

`ResponseKind` is one of these variants.

| Variant | v0 | Unlock rule |
| --- | --- | --- |
| `Mcq` | Yes | Four choices, one correct index 0..3. Wrong maps to `Rating::Again`. Right maps to `Rating::Good`. Then unlock |
| `TypedRecall` | No | Later |
| `RevealThenGrade` | No | Later. Default for raw Anki HTML if imported |

`Rating` is `Again`, `Hard`, `Good`, or `Easy`. There is no fifth grade.

## Card and memory

A `Card` has `CardId`, optional `NoteId`, `queue`, `MemoryState`, `due`, `reps`, `lapses`.

`queue` is `New`, `Learning`, `Review`, or `Relearning`. Suspended and buried are not queues that can appear as a lock prompt. Those cards are absent from due lists.

`MemoryState` is FSRS `stability` and `difficulty` plus last review time. SM-2 ease is a different object. Do not store both on one card.

`ReviewLog` is append-only. Fields are `CardId`, `PromptId`, `Rating`, elapsed, timestamps, pre state, post state.

## Gate session

`GatePhase` is a sum type.

| Variant | Payload |
| --- | --- |
| `Idle` | None |
| `Locked` | `SessionId`, `PromptId`, started at |
| `Loan` | `SessionId`, until |
| `Unlocked` | Terminal. Do not persist as open |

At most one `Locked` row exists.

Crash with `Locked` on disk reloads as `Locked` and the lock backend must freeze again.

## Bail

`Bail` has method, timestamps, and cost paid. It is a session event.

A `Bail` constructor does not accept a `Rating`. A `ReviewLog` constructor does not accept a bail method.

Loan after bail is `GatePhase::Loan`. When `until` passes, the daemon starts a new `Locked` with a new `PromptId`.

## Sources

A source yields `Prompt` values. v0 source is a local table of MCQ rows.

Anki `.apkg` is a later importer. It copies fields into `Prompt`. It does not call AnkiConnect `answerCards` from a freeze.

## Illegal combinations

These must not be representable.

- `Unlocked` stored as an open session
- `AwaitingGrade` together with `ResponseKind::Mcq` in v0
- `Rating` without `PromptId` and `reviewed_at`
- `MemoryState` on a note
- Due prompt with suspended or buried origin
- `Bail` that writes `ReviewLog`
- Two `Locked` rows
- Local FSRS write and AnkiConnect `answerCards` both marked synced without a single-writer token
