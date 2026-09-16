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
    pub item_id: ItemId,
    pub chosen_index: ChoiceIndex,
    pub correct: bool,
    pub answered_at: DateTime<Utc>,
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
        let eligible: Vec<ItemId> = self.deck_items();
        eligible.choose(rng).copied()
    }

    pub fn get(&self, id: ItemId) -> Option<&Item> {
        self.items.iter().find(|item| item.id == id)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use rand::thread_rng;

    use super::*;
    use crate::ids::ItemId;

    fn sample_item(id: ItemId, suspended: bool) -> Item {
        Item::new(
            id,
            "stem",
            McqChoices::new(["a".into(), "b".into(), "c".into(), "d".into()]),
            ChoiceIndex::try_new(2).expect("index"),
            suspended,
            None,
        )
    }

    #[test]
    fn wrong_mcq_index_records_incorrect_answer() {
        let item = sample_item(ItemId::new(), false);
        let chosen = ChoiceIndex::try_new(0).expect("index");
        let answer = item.answer_for_choice(chosen, Utc::now());
        assert!(!answer.correct);
        assert_eq!(answer.item_id, item.id);
    }

    #[test]
    fn correct_mcq_index_records_correct_answer() {
        let item = sample_item(ItemId::new(), false);
        let chosen = ChoiceIndex::try_new(2).expect("index");
        let answer = item.answer_for_choice(chosen, Utc::now());
        assert!(answer.correct);
    }

    #[test]
    fn pick_item_excludes_suspended() {
        let active = ItemId::new();
        let suspended = ItemId::new();
        let deck = Deck::new(vec![sample_item(active, false), sample_item(suspended, true)]);
        assert_eq!(deck.deck_items(), vec![active]);
        for _ in 0..20 {
            let picked = deck.pick_item(&mut thread_rng()).expect("pick");
            assert_eq!(picked, active);
        }
    }
}
