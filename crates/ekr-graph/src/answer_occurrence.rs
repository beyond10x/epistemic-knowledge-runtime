//! Checked adapter from generated answer metadata to the historical revision envelope.
use ekr_core::contract_data::EkrKernelAttentionAnsweredPayload;
use ekr_core::{ContentHash, EventId, RevisionId, RevisionNumber, TransactionId};
use serde::{Deserialize, Serialize};

/// Generated answer metadata with checked scalar ingress. It cannot be mutated after admission.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    try_from = "EkrKernelAttentionAnsweredPayload",
    into = "EkrKernelAttentionAnsweredPayload"
)]
pub struct AnswerOccurrence(EkrKernelAttentionAnsweredPayload);
// The generated fields are strings and an integer checked into u64; all are reflexively equal.
impl Eq for AnswerOccurrence {}
impl TryFrom<EkrKernelAttentionAnsweredPayload> for AnswerOccurrence {
    type Error = String;
    fn try_from(value: EkrKernelAttentionAnsweredPayload) -> Result<Self, Self::Error> {
        value
            .answer_id
            .0
            .parse::<EventId>()
            .map_err(|e| e.to_string())?;
        value
            .transaction_id
            .0
            .parse::<TransactionId>()
            .map_err(|e| e.to_string())?;
        value
            .revision_id
            .0
            .parse::<RevisionId>()
            .map_err(|e| e.to_string())?;
        value
            .knowledge_root
            .0
            .parse::<ContentHash>()
            .map_err(|e| e.to_string())?;
        value.number.0.as_u64().ok_or("invalid answer revision")?;
        Ok(Self(value))
    }
}
impl From<AnswerOccurrence> for EkrKernelAttentionAnsweredPayload {
    fn from(value: AnswerOccurrence) -> Self {
        value.0
    }
}
impl AnswerOccurrence {
    /// The signed human decision's UUID, reused as answer identity.
    #[must_use]
    pub fn answer_id(&self) -> EventId {
        self.0.answer_id.0.parse().expect("checked answer identity")
    }
    /// The derived ordinary transaction's identity.
    #[must_use]
    pub fn transaction_id(&self) -> TransactionId {
        self.0
            .transaction_id
            .0
            .parse()
            .expect("checked transaction identity")
    }
    /// The revision created by the answer.
    #[must_use]
    pub fn revision_id(&self) -> RevisionId {
        self.0
            .revision_id
            .0
            .parse()
            .expect("checked revision identity")
    }
    /// The revision's position.
    #[must_use]
    pub fn number(&self) -> RevisionNumber {
        RevisionNumber::new(self.0.number.0.as_u64().expect("checked revision"))
    }
    /// The resulting canonical knowledge address.
    #[must_use]
    pub fn knowledge_root(&self) -> ContentHash {
        self.0
            .knowledge_root
            .0
            .parse()
            .expect("checked knowledge address")
    }
}
impl ekr_core::canonical::Canonical for AnswerOccurrence {
    fn encode(&self, out: &mut ekr_core::canonical::Encoder) {
        self.answer_id().encode(out);
        self.transaction_id().encode(out);
        self.revision_id().encode(out);
        self.number().encode(out);
        self.knowledge_root().encode(out);
    }
}

/// Checked generated human-answer identity for durable command slots.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    try_from = "ekr_core::contract_data::EkrKernelHumanAnswerId",
    into = "ekr_core::contract_data::EkrKernelHumanAnswerId"
)]
pub struct HumanAnswerId(ekr_core::contract_data::EkrKernelHumanAnswerId);
// Its sole generated field is a checked UUID string.
impl Eq for HumanAnswerId {}
impl TryFrom<ekr_core::contract_data::EkrKernelHumanAnswerId> for HumanAnswerId {
    type Error = String;
    fn try_from(
        value: ekr_core::contract_data::EkrKernelHumanAnswerId,
    ) -> Result<Self, Self::Error> {
        value.0.parse::<EventId>().map_err(|e| e.to_string())?;
        Ok(Self(value))
    }
}
impl From<HumanAnswerId> for ekr_core::contract_data::EkrKernelHumanAnswerId {
    fn from(value: HumanAnswerId) -> Self {
        value.0
    }
}
impl From<EventId> for HumanAnswerId {
    fn from(value: EventId) -> Self {
        Self(ekr_core::contract_data::EkrKernelHumanAnswerId(
            value.to_string(),
        ))
    }
}
