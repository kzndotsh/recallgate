use chrono::{DateTime, Utc};
use rand::seq::SliceRandom;

use crate::ids::ItemId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McqChoices {
    inner: [String; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChoiceIndex(u8);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub id: ItemId,
    pub stem: String,
    pub choices: McqChoices,
    pub correct_index: ChoiceIndex,
    pub suspended: bool,
    pub external_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    item_id: ItemId,
    chosen_index: ChoiceIndex,
    correct: bool,
    answered_at: DateTime<Utc>,
}

impl Answer {
    pub fn item_id(&self) -> ItemId {
        self.item_id
    }

    pub fn chosen_index(&self) -> ChoiceIndex {
        self.chosen_index
    }

    pub fn correct(&self) -> bool {
        self.correct
    }

    pub fn answered_at(&self) -> DateTime<Utc> {
        self.answered_at
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deck {
    items: Vec<Item>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemError {
    InvalidChosenIndex,
}

impl McqChoices {
    pub fn new(choices: [String; 4]) -> Self {
        Self { inner: choices }
    }

    pub fn as_slice(&self) -> &[String] {
        &self.inner
    }
}

impl ChoiceIndex {
    pub fn try_new(index: u8) -> Result<Self, ItemError> {
        if index > 3 {
            return Err(ItemError::InvalidChosenIndex);
        }
        Ok(Self(index))
    }

    pub fn index(self) -> u8 {
        self.0
    }
}

impl Item {
    pub fn new(
        id: ItemId,
        stem: impl Into<String>,
        choices: McqChoices,
        correct_index: ChoiceIndex,
        suspended: bool,
        external_ref: Option<String>,
    ) -> Self {
        Self { id, stem: stem.into(), choices, correct_index, suspended, external_ref }
    }

    pub fn answer_for_choice(
        &self,
        chosen_index: ChoiceIndex,
        answered_at: DateTime<Utc>,
    ) -> Answer {
        let correct = chosen_index == self.correct_index;
        Answer { item_id: self.id, chosen_index, correct, answered_at }
    }
}

impl Deck {
    pub fn new(items: Vec<Item>) -> Self {
        Self { items }
    }

    pub fn items(&self) -> &[Item] {
        &self.items
    }

    pub fn deck_items(&self) -> Vec<ItemId> {
        self.items.iter().filter(|item| !item.suspended).map(|item| item.id).collect()
    }

    pub fn pick_item<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Option<ItemId> {
        self.items
            .iter()
            .filter(|item| !item.suspended)
            .map(|item| item.id)
            .collect::<Vec<_>>()
            .choose(rng)
            .copied()
    }

    pub fn get(&self, id: ItemId) -> Option<&Item> {
        self.items.iter().find(|item| item.id == id)
    }
}
