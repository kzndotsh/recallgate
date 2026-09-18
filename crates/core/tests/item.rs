use chrono::Utc;
use rand::thread_rng;
use recallgate_core::{ChoiceIndex, Deck, Item, ItemId, McqChoices};

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
    assert!(!answer.correct());
    assert_eq!(answer.item_id(), item.id);
}

#[test]
fn correct_mcq_index_records_correct_answer() {
    let item = sample_item(ItemId::new(), false);
    let chosen = ChoiceIndex::try_new(2).expect("index");
    let answer = item.answer_for_choice(chosen, Utc::now());
    assert!(answer.correct());
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
