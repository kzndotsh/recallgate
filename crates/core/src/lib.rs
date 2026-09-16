//! Recall Gate core library. Domain types per `docs/domain.md`.

mod abort;
mod cadence;
mod gate;
mod ids;
mod item;

pub use abort::Abort;
pub use cadence::GateCadence;
pub use gate::{CooldownState, GateError, GatePhase, GateState, LockedState};
pub use ids::{GateId, IdError, ItemId};
pub use item::{Answer, ChoiceIndex, Deck, Item, ItemError, McqChoices};
