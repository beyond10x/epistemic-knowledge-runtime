//! Generated observation command obligations, with the original provider fault preserved.
//!
//! Wire/semantic conversion is explicit; both models are generated. Domain refusals are
//! generated outcomes. Infrastructure failure never becomes a successful or refused domain
//! outcome: the invocation retains the original store error for its caller.
use crate::Commit;
use ekr_core::contract_data as wire;
use ekr_core::contracts::obligation::UnmetObligation;
use ekr_core::contracts::observe::obligations::{
    ImportObservationBehavior, ListObservationsBehavior, ShowObservationBehavior,
};
use ekr_core::contracts::{graph, kernel, observe, primitives};
use ekr_core::{ObservationId, Timestamp};
use ekr_store::{ObjectStore, ObservationRetention, RevisionLog, StoreError};

struct Behavior<'a, S: RevisionLog + ObjectStore> {
    commit: &'a Commit<S>,
    at: Timestamp,
    fault: Option<StoreError>,
}

impl<'a, S: RevisionLog + ObjectStore> Behavior<'a, S> {
    fn new(commit: &'a Commit<S>, at: Timestamp) -> Self {
        Self {
            commit,
            at,
            fault: None,
        }
    }

    fn finish<T>(&mut self, result: Result<T, UnmetObligation>) -> Result<T, StoreError> {
        if let Some(error) = self.fault.take() {
            return Err(error);
        }
        result.map_err(|error| StoreError::Document(error.to_string()))
    }

    fn failed(&mut self, error: StoreError, source: &'static str) -> UnmetObligation {
        self.fault = Some(error);
        UnmetObligation {
            capability: "available observation persistence",
            source,
        }
    }

    fn refusal(&mut self, error: StoreError) -> observe::KnowledgeRefused {
        let refusal = observe::KnowledgeRefused {
            code: "observation-refused".into(),
            reason: error.to_string(),
        };
        self.fault = Some(error);
        refusal
    }
}

fn optional<T: Clone>(value: &wire::EssPresence<T>) -> Option<T> {
    match value {
        wire::EssPresence::Absent => None,
        wire::EssPresence::Present(value) => Some(value.clone()),
    }
}
fn presence<T>(value: Option<T>) -> wire::EssPresence<T> {
    value.map_or(wire::EssPresence::Absent, wire::EssPresence::Present)
}

fn record(value: &wire::EkrGraphObservationRecord) -> graph::ObservationRecord {
    use graph::ObservationKind as K;
    use wire::EkrGraphObservationKind as W;
    graph::ObservationRecord {
        observation_id: graph::ObservationId(primitives::Uuid(value.observation_id.0.clone())),
        source: value.source.clone(),
        source_native_id: optional(&value.source_native_id),
        kind: match *value.kind {
            W::V0 => K::ApiResponse,
            W::V1 => K::Blob,
            W::V2 => K::DatabaseRecord,
            W::V3 => K::Document,
            W::V4 => K::FeedItem,
            W::V5 => K::GitDiff,
            W::V6 => K::GraphFragment,
            W::V7 => K::MessageBatch,
        },
        content_hash: kernel::ContentHash(value.content_hash.0.clone()),
        captured_at: primitives::Timestamp(value.captured_at.clone()),
    }
}
fn wire_record(value: graph::ObservationRecord) -> wire::EkrGraphObservationRecord {
    use graph::ObservationKind as K;
    use wire::EkrGraphObservationKind as W;
    wire::EkrGraphObservationRecord {
        observation_id: Box::new(wire::EkrGraphObservationId(value.observation_id.0 .0)),
        source: value.source,
        source_native_id: presence(value.source_native_id),
        kind: Box::new(match value.kind {
            K::ApiResponse => W::V0,
            K::Blob => W::V1,
            K::DatabaseRecord => W::V2,
            K::Document => W::V3,
            K::FeedItem => W::V4,
            K::GitDiff => W::V5,
            K::GraphFragment => W::V6,
            K::MessageBatch => W::V7,
        }),
        content_hash: Box::new(wire::EkrKernelContentHash(value.content_hash.0)),
        captured_at: value.captured_at.0,
    }
}
fn key(value: &wire::EkrObserveObservationIdempotencyKey) -> observe::ObservationIdempotencyKey {
    observe::ObservationIdempotencyKey {
        source: value.source.clone(),
        source_native_id: optional(&value.source_native_id),
        content_hash: kernel::ContentHash(value.content_hash.0.clone()),
    }
}
fn wire_key(
    value: observe::ObservationIdempotencyKey,
) -> wire::EkrObserveObservationIdempotencyKey {
    wire::EkrObserveObservationIdempotencyKey {
        source: value.source,
        source_native_id: presence(value.source_native_id),
        content_hash: Box::new(wire::EkrKernelContentHash(value.content_hash.0)),
    }
}
fn decode_payload(value: &str) -> Result<Vec<u8>, StoreError> {
    ekr_core::bytes::decode(value).map_err(|error| StoreError::Document(error.to_string()))
}

impl<S: RevisionLog + ObjectStore + ObservationRetention> ImportObservationBehavior
    for Behavior<'_, S>
{
    fn import_observation(
        &mut self,
        input: observe::ImportObservation,
    ) -> Result<observe::ImportObservationOutcome, UnmetObligation> {
        let input = wire::EkrObserveObservationImport {
            observation: Box::new(wire_record(input.document.observation)),
            key: Box::new(wire_key(input.document.key)),
            payload: ekr_core::bytes::encode(&input.document.payload),
        };
        match self.commit.retain_observation(&input, self.at) {
            Ok(receipt) => Ok(observe::ImportObservationOutcome::Answered {
                import_observation_result: observe::ImportObservationResult {
                    receipt: observe::ObservationImportReceipt {
                        outcome: if receipt.already_retained {
                            observe::ObservationRetentionOutcome::AlreadyRetained
                        } else {
                            observe::ObservationRetentionOutcome::Retained
                        },
                        observation_id: graph::ObservationId(primitives::Uuid(
                            receipt.observation_id.0,
                        )),
                        content_hash: kernel::ContentHash(receipt.content_hash.0),
                        already_retained: receipt.already_retained,
                    },
                },
            }),
            Err(error @ (StoreError::Document(_) | StoreError::PublicationInputConflict)) => {
                Ok(observe::ImportObservationOutcome::Refused {
                    error: self.refusal(error),
                })
            }
            Err(error) => Err(self.failed(error, "ekr.observe.ImportObservation")),
        }
    }
}
impl<S: RevisionLog + ObjectStore + ObservationRetention> ListObservationsBehavior
    for Behavior<'_, S>
{
    fn list_observations(
        &mut self,
        _: observe::ListObservations,
    ) -> Result<observe::ListObservationsOutcome, UnmetObligation> {
        match self.commit.retained_observation_records() {
            Ok(records) => Ok(observe::ListObservationsOutcome::Listed {
                observations_listed: observe::ObservationsListed {
                    observations: records.iter().map(record).collect(),
                },
            }),
            Err(error) => Err(self.failed(error, "ekr.observe.ListObservations")),
        }
    }
}
impl<S: RevisionLog + ObjectStore + ObservationRetention> ShowObservationBehavior
    for Behavior<'_, S>
{
    fn show_observation(
        &mut self,
        input: observe::ShowObservation,
    ) -> Result<observe::ShowObservationOutcome, UnmetObligation> {
        let result = input
            .observation_id
            .0
             .0
            .parse::<ObservationId>()
            .map_err(|error| StoreError::Document(error.to_string()))
            .and_then(|id| self.commit.retained_observation(id))
            .and_then(|held| {
                Ok(observe::RetainedObservationRead {
                    observation: record(&held.observation),
                    key: key(&held.key),
                    payload: decode_payload(&held.payload)?,
                })
            });
        match result {
            Ok(retained) => Ok(observe::ShowObservationOutcome::Shown {
                observation_shown: observe::ObservationShown { retained },
            }),
            Err(error @ StoreError::Document(_)) => Ok(observe::ShowObservationOutcome::Refused {
                error: self.refusal(error),
            }),
            Err(error) => Err(self.failed(error, "ekr.observe.ShowObservation")),
        }
    }
}

pub(super) fn import<S: RevisionLog + ObjectStore + ObservationRetention>(
    commit: &Commit<S>,
    input: &wire::EkrObserveObservationImport,
    at: Timestamp,
) -> Result<wire::EkrObserveObservationImportReceipt, StoreError> {
    let mut behavior = Behavior::new(commit, at);
    let result = behavior.import_observation(observe::ImportObservation {
        document: observe::ObservationImport {
            observation: record(&input.observation),
            key: key(&input.key),
            payload: decode_payload(&input.payload)?,
        },
    });
    match behavior.finish(result)? {
        observe::ImportObservationOutcome::Answered {
            import_observation_result,
        } => {
            let receipt = import_observation_result.receipt;
            Ok(wire::EkrObserveObservationImportReceipt {
                observation_id: Box::new(wire::EkrGraphObservationId(receipt.observation_id.0 .0)),
                content_hash: Box::new(wire::EkrKernelContentHash(receipt.content_hash.0)),
                already_retained: receipt.already_retained,
                outcome: Box::new(match receipt.outcome {
                    observe::ObservationRetentionOutcome::Retained => {
                        wire::EkrObserveObservationRetentionOutcome::V1
                    }
                    observe::ObservationRetentionOutcome::AlreadyRetained => {
                        wire::EkrObserveObservationRetentionOutcome::V0
                    }
                }),
            })
        }
        observe::ImportObservationOutcome::Refused { error } => {
            Err(StoreError::Document(error.reason))
        }
    }
}
pub(super) fn list<S: RevisionLog + ObjectStore + ObservationRetention>(
    commit: &Commit<S>,
) -> Result<Vec<wire::EkrGraphObservationRecord>, StoreError> {
    let mut behavior = Behavior::new(commit, Timestamp::EPOCH);
    let result = behavior.list_observations(observe::ListObservations {});
    let observe::ListObservationsOutcome::Listed {
        observations_listed,
    } = behavior.finish(result)?;
    Ok(observations_listed
        .observations
        .into_iter()
        .map(wire_record)
        .collect())
}
pub(super) fn show<S: RevisionLog + ObjectStore + ObservationRetention>(
    commit: &Commit<S>,
    id: ObservationId,
) -> Result<wire::EkrObserveRetainedObservationRead, StoreError> {
    let mut behavior = Behavior::new(commit, Timestamp::EPOCH);
    let result = behavior.show_observation(observe::ShowObservation {
        observation_id: graph::ObservationId(primitives::Uuid(id.to_string())),
    });
    match behavior.finish(result)? {
        observe::ShowObservationOutcome::Shown { observation_shown } => {
            let retained = observation_shown.retained;
            Ok(wire::EkrObserveRetainedObservationRead {
                observation: Box::new(wire_record(retained.observation)),
                key: Box::new(wire_key(retained.key)),
                payload: ekr_core::bytes::encode(&retained.payload),
            })
        }
        observe::ShowObservationOutcome::Refused { error } => {
            Err(StoreError::Document(error.reason))
        }
    }
}
