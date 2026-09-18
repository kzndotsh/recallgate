use chrono::Utc;
use recallgate_core::{Abort, Answer, ChoiceIndex, GateId, Item, ItemId, McqChoices};

#[test]
fn abort_is_not_an_answer() {
    let abort = Abort::new(GateId::new(), "chord", Utc::now(), 1);
    let item = Item::new(
        ItemId::new(),
        "stem",
        McqChoices::new(["a".into(), "b".into(), "c".into(), "d".into()]),
        ChoiceIndex::try_new(2).expect("index"),
        false,
        None,
    );
    let answer = item.answer_for_choice(ChoiceIndex::try_new(0).expect("index"), Utc::now());
    let _abort_only: Abort = abort;
    let _answer_only: Answer = answer;
}
