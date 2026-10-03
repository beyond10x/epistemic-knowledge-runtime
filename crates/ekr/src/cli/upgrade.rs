//! Explicit upgrade transport. Trust selection and proof verification stay in the kernel.
use super::{input, render, Access, Printed};
use crate::exit::Failure;
use clap::Subcommand;
use ekr_core::contract_data::{EkrKernelAuthorityUpgradeApplication, EkrKernelReviewerTrustPolicy};
use ekr_kernel::{human_review, CommitError, PersistenceError, Runtime};
use std::io::Read;
use std::path::PathBuf;

/// Preview or apply the explicit, signed authority transition of an existing store.
#[derive(Debug, Subcommand)]
pub enum UpgradeCommand {
    /// Preview the supported knowledge authority using a public ReviewerTrustPolicy JSON file.
    Preview {
        /// Policy document, or `-` for stdin. It must match independent system provisioning.
        policy: PathBuf,
    },
    /// Apply an AuthorityUpgradeApplication JSON document reviewed and signed by a human.
    Apply {
        /// Document, or `-` for stdin. The limit is eight MiB; it contains no signing key.
        document: PathBuf,
    },
}
impl UpgradeCommand {
    pub(super) fn access(&self) -> Access {
        match self {
            Self::Preview { .. } => Access::Read,
            Self::Apply { .. } => Access::Write,
        }
    }
}
fn refused(reason: impl std::fmt::Display) -> Failure {
    Failure::refused("ekr.kernel.KnowledgeRefused", reason)
}
fn review_error(error: ekr_core::contracts::kernel::KnowledgeRefused) -> Failure {
    refused(format!("{}: {}", error.code, error.reason))
}
fn failure(error: CommitError) -> Failure {
    match error {
        CommitError::Store(PersistenceError::Document(ref detail))
            if detail.contains("authority-upgrade:") || detail.contains("upgrade-preview-") =>
        {
            refused(detail)
        }
        other => other.into(),
    }
}
fn document<T: serde::de::DeserializeOwned>(
    path: &std::path::Path,
    stdin: &mut dyn Read,
) -> Result<T, Failure> {
    let mut bytes = Vec::new();
    input::open(path, stdin)?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(Failure::fault)?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err(refused("upgrade input exceeds eight MiB"));
    }
    serde_json::from_slice(&bytes).map_err(refused)
}
pub(super) fn run(
    command: UpgradeCommand,
    runtime: &Runtime,
    now: &dyn Fn() -> ekr_core::Timestamp,
    stdin: &mut dyn Read,
) -> Result<Printed, Failure> {
    match command {
        UpgradeCommand::Preview { policy } => {
            let wire: EkrKernelReviewerTrustPolicy = document(&policy, stdin)?;
            let policy = human_review::policy_from_document(&wire).map_err(review_error)?;
            render(&runtime.preview_upgrade(&policy).map_err(failure)?)
        }
        UpgradeCommand::Apply { document: path } => {
            let input: EkrKernelAuthorityUpgradeApplication = document(&path, stdin)?;
            let policy = human_review::policy_from_document(&input.policy).map_err(review_error)?;
            let proof =
                human_review::proof_from_document(&input.human_proof).map_err(review_error)?;
            let statement = ekr_core::bytes::decode(&input.statement).map_err(refused)?;
            render(
                &runtime
                    .apply_upgrade(&input.preview, &policy, &proof, &statement, now)
                    .map_err(failure)?,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::session::{fixture, InProcess};
    use ekr_core::contract_data::*;
    use ekr_core::contracts::kernel as m;
    use ekr_kernel::SeedDocument;
    use ekr_sdk::knowledge::Knowledge;
    use ring::signature::{Ed25519KeyPair, KeyPair};

    #[test]
    fn sdk_upgrade_uses_the_real_session_kernel_and_exact_external_signature() {
        for backend in fixture::BACKENDS {
            let directory = tempfile::tempdir().unwrap();
            let store = fixture::seeded(directory.path(), backend, "upgrade");
            let runtime = store.open().unwrap();
            let seed = SeedDocument::from_yaml(
                &std::fs::read_to_string(directory.path().join("seed.yaml")).unwrap(),
            )
            .unwrap();
            let seeded = runtime.seed(seed, || panic!("seed retry")).unwrap();
            let key = Ed25519KeyPair::from_seed_unchecked(&[23; 32]).unwrap();
            let audience = serde_json::json!({"tenant": store.host.tenant, "seed_anchor": seeded.seed_hash.to_string()});
            let wire_policy: EkrKernelReviewerTrustPolicy = serde_json::from_value(serde_json::json!({
                "format": "ekr.reviewer-trust/1", "audience": audience,
                "keys": [{"key_digest": human_review::digest(key.public_key().as_ref()).to_string(),
                    "algorithm": "Ed25519", "public_key": ekr_core::bytes::encode(key.public_key().as_ref()),
                    "operator": {"actor": store.host.context.operator.to_string(), "authentication_subject": "fixture-human"},
                    "scopes": ["UpgradeAuthority"]}]
            })).unwrap();
            let policy = human_review::policy_from_document(&wire_policy).unwrap();
            let digest =
                human_review::digest(&human_review::policy_bytes(&policy).unwrap()).to_string();
            let binding = m::TrustedReviewHostBinding {
                audience: policy.audience.clone(),
                reviewer_policy_digest: m::ContentHash(digest.clone()),
            };
            let retry = std::cell::Cell::new(false);
            let now = || {
                assert!(!retry.get(), "a completed retry must not call the clock");
                ekr_core::Timestamp::from_millis(1)
            };
            let mut unprovisioned = InProcess::new(store.clone(), runtime, &now);
            let refusal = Knowledge::new(&mut unprovisioned)
                .preview_upgrade(&wire_policy)
                .unwrap_err();
            assert!(
                matches!(refusal, ekr_sdk::read::ReadError::Refused { .. }),
                "{refusal}"
            );
            unprovisioned.close();
            let runtime = store
                .open()
                .unwrap()
                .with_review_authority(binding.clone())
                .unwrap();
            let mut transport = InProcess::new(store.clone(), runtime, &now);
            let mut knowledge = Knowledge::new(&mut transport);
            let preview = knowledge.preview_upgrade(&wire_policy).unwrap();
            let statement = b"reviewed the complete preview";
            let mut proof: EkrKernelSignedHumanDecision = serde_json::from_value(serde_json::json!({
                "algorithm": "Ed25519", "signature": ekr_core::bytes::encode(&[0;64]),
                "intent": {"format": "ekr.human-decision/1", "decision_id": ekr_core::EventId::mint().to_string(),
                    "audience": audience, "reviewer_policy_digest": digest,
                    "signer_key_digest": human_review::digest(key.public_key().as_ref()).to_string(),
                    "statement_digest": human_review::digest(statement).to_string(),
                    "target": {"kind": "UpgradeAuthority", "value": {"preview_digest": preview.preview_digest,
                        "reviewer_policy_digest": digest}}}
            })).unwrap();
            let semantic = human_review::proof_from_document(&proof).unwrap();
            proof.signature = ekr_core::bytes::encode(
                key.sign(&human_review::signing_bytes(&semantic.intent).unwrap())
                    .as_ref(),
            );
            let input = EkrKernelAuthorityUpgradeApplication {
                preview: Box::new(preview),
                policy: Box::new(wire_policy),
                human_proof: Box::new(proof),
                statement: ekr_core::bytes::encode(statement),
            };
            let mut forged = input.clone();
            forged.human_proof.signature = ekr_core::bytes::encode(&[0; 64]);
            assert!(matches!(
                knowledge.apply_upgrade(&forged),
                Err(ekr_sdk::read::ReadError::Refused { .. })
            ));
            let receipt = knowledge.apply_upgrade(&input).unwrap();
            retry.set(true);
            assert_eq!(knowledge.apply_upgrade(&input).unwrap(), receipt);
            assert!(knowledge.attention().unwrap().is_empty());
            transport.close();
            let mut reopened = store
                .open()
                .unwrap()
                .with_review_authority(binding)
                .unwrap();
            reopened.set_full_replay(true);
            assert_eq!(reopened.head().unwrap().unwrap().revision.get(), 1);
        }
    }
}
