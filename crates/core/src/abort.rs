use chrono::{DateTime, Utc};

use crate::ids::GateId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Abort {
    pub gate_id: GateId,
    pub method: String,
    pub at: DateTime<Utc>,
    pub cost_paid: u32,
}

impl Abort {
    pub fn new(
        gate_id: GateId,
        method: impl Into<String>,
        at: DateTime<Utc>,
        cost_paid: u32,
    ) -> Self {
        Self { gate_id, method: method.into(), at, cost_paid }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::GateId;
    use crate::item::{Answer, ChoiceIndex, Item, McqChoices};

    #[test]
    fn abort_is_not_an_answer() {
        let abort = Abort::new(GateId::new(), "chord", Utc::now(), 1);
        let item = Item::new(
            crate::ids::ItemId::new(),
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
}
