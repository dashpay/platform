//! One paid erase attempt, with its eligibility and observations owned by Rust.

use std::collections::BTreeSet;
use std::future::Future;
use std::sync::{Arc, Mutex};

use dash_sdk::drive::query::document_history_drive_query::{
    DocumentHistoryFilter, DocumentHistoryLifecycle, DocumentHistoryState,
};
use dash_sdk::platform::documents::document_history_query::DocumentHistoryQuery;
use dash_sdk::platform::documents::transitions::DocumentEraseTransitionBuilder;
use dash_sdk::platform::Fetch;
use dash_sdk::query_types::DocumentHistory;
use dash_sdk::Error as SdkError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::DocumentType;
use dpp::document::DocumentV0Getters;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey, KeyType};
use dpp::prelude::{DataContract, Identifier};

use super::document::SignerRef;
use super::{usable_authentication_key, IdentityWallet};
use crate::error::{preserve_signer_key_unavailable_or, PlatformWalletError};

/// A proved lifecycle, or a read that could not establish one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentLifecycleObservation {
    Observed(DocumentHistoryLifecycle),
    Unavailable,
}

/// Why no paid transition was submitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentEraseNotSubmittedReason {
    Active,
    Absent,
    NotNewestOwner { owner: Identifier },
    PreflightUnavailable,
}

/// Submission and observations are separate: an unavailable closing read does
/// not turn a submitted erase into a submission failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentEraseOutcome {
    NotSubmitted {
        reason: DocumentEraseNotSubmittedReason,
        before: DocumentLifecycleObservation,
    },
    Submitted {
        before: DocumentLifecycleObservation,
        after: DocumentLifecycleObservation,
    },
}

#[derive(Debug, PartialEq, Eq)]
enum DocumentEraseDecision {
    NotSubmitted(DocumentEraseNotSubmittedReason),
    Submit,
}

fn decide(
    before: &DocumentLifecycleObservation,
    submitter: Identifier,
    newest_owner: Option<Identifier>,
) -> DocumentEraseDecision {
    use DocumentEraseNotSubmittedReason as Reason;
    let reason = match before {
        DocumentLifecycleObservation::Observed(lifecycle) => match lifecycle.state {
            DocumentHistoryState::Active => Reason::Active,
            DocumentHistoryState::Absent => Reason::Absent,
            DocumentHistoryState::Erasing => return DocumentEraseDecision::Submit,
            DocumentHistoryState::Deleted => match newest_owner {
                Some(owner) if owner == submitter => return DocumentEraseDecision::Submit,
                Some(owner) => Reason::NotNewestOwner { owner },
                None => Reason::PreflightUnavailable,
            },
        },
        DocumentLifecycleObservation::Unavailable => Reason::PreflightUnavailable,
    };
    DocumentEraseDecision::NotSubmitted(reason)
}

/// Shared by cloned wallet handles, including calls that outlive a host await.
#[derive(Debug, Default)]
pub(crate) struct DocumentEraseGate(Mutex<BTreeSet<(Identifier, Identifier)>>);

impl DocumentEraseGate {
    fn enter(
        &self,
        contract: Identifier,
        document: Identifier,
    ) -> Result<DocumentEraseGuard<'_>, PlatformWalletError> {
        if !self
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert((contract, document))
        {
            return Err(PlatformWalletError::DocumentEraseBusy);
        }
        Ok(DocumentEraseGuard {
            gate: self,
            key: (contract, document),
        })
    }
}

struct DocumentEraseGuard<'a> {
    gate: &'a DocumentEraseGate,
    key: (Identifier, Identifier),
}

impl Drop for DocumentEraseGuard<'_> {
    fn drop(&mut self) {
        self.gate
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&self.key);
    }
}

fn select_erase_key(
    identity: &Identity,
    document_type: &DocumentType,
    contract_id: Identifier,
    signer: &impl Signer<IdentityPublicKey>,
) -> Result<IdentityPublicKey, PlatformWalletError> {
    usable_authentication_key(
        identity,
        signer,
        contract_id,
        document_type.name(),
        &document_type
            .security_level_requirement()
            .document_signing_levels(),
        &[KeyType::ECDSA_SECP256K1],
    )
    .map_err(PlatformWalletError::Sdk)?
    .cloned()
    .ok_or_else(|| {
        PlatformWalletError::InvalidIdentityData("No eligible document erase signing key".into())
    })
}

fn observation(history: Option<&DocumentHistory>) -> DocumentLifecycleObservation {
    history
        .and_then(|history| history.lifecycle.clone())
        .map(DocumentLifecycleObservation::Observed)
        .unwrap_or(DocumentLifecycleObservation::Unavailable)
}

fn validate_erasable_type(document_type: &DocumentType) -> Result<(), PlatformWalletError> {
    if !document_type.documents_keep_history()
        || !document_type.documents_can_be_erased()
        || document_type.find_contested_index().is_some()
    {
        return Err(PlatformWalletError::InvalidIdentityData(format!(
            "Document type {:?} does not permit history erasure",
            document_type.name()
        )));
    }
    Ok(())
}

impl IdentityWallet {
    /// Fetch/register the contract, establish a free proved preflight, select a
    /// usable signer key, and submit at most once. Unreadable preflight fails
    /// closed. A deleted document requires the newest retained revision owner;
    /// any loaded identity may continue an erasing document. Key selection uses
    /// the real type requirement and the supplied signer. An unconfirmed
    /// submission is an error whose outcome is unknown;
    /// neither that error nor a successful return triggers automatic retries.
    /// An unproved consensus refusal after broadcast also has an unknown outcome.
    pub async fn erase_document_and_observe_with_signer<S>(
        &self,
        submitting_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
        document_id: &Identifier,
        signer: &S,
    ) -> Result<DocumentEraseOutcome, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        self.erase_and_observe_with_submit(
            submitting_identity_id,
            contract_id,
            document_type_name,
            document_id,
            signer,
            |contract, key| async move {
                let builder = DocumentEraseTransitionBuilder::new(
                    contract,
                    document_type_name.to_owned(),
                    *document_id,
                    *submitting_identity_id,
                );
                self.sdk
                    .document_erase(builder, &key, &SignerRef(signer))
                    .await
                    .map(|_| ())
            },
        )
        .await
    }

    async fn read_erase_history(&self, query: DocumentHistoryQuery) -> Option<DocumentHistory> {
        match DocumentHistory::fetch(&self.sdk, query).await {
            Ok(history) => history,
            Err(error) => {
                tracing::warn!(%error, "Document erase lifecycle observation unavailable");
                None
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn erase_and_observe_with_submit<S, F, Fut>(
        &self,
        submitting_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
        document_id: &Identifier,
        signer: &S,
        submit: F,
    ) -> Result<DocumentEraseOutcome, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
        F: FnOnce(Arc<DataContract>, IdentityPublicKey) -> Fut,
        Fut: Future<Output = Result<(), SdkError>>,
    {
        let _guard = self.document_erase_gate.enter(*contract_id, *document_id)?;
        let contract = self
            .fetch_contract_arc_for_document_op(contract_id, document_type_name)
            .await?;
        let document_type = contract
            .document_type_cloned_for_name(document_type_name)
            .map_err(|e| PlatformWalletError::InvalidIdentityData(e.to_string()))?;
        validate_erasable_type(&document_type)?;

        let query = DocumentHistoryQuery {
            data_contract_id: *contract_id,
            document_type_name: document_type_name.to_owned(),
            document_id: *document_id,
            filter: DocumentHistoryFilter::StartAtTime(0),
            limit: Some(1),
        };
        let history = self.read_erase_history(query.clone()).await;
        let before = observation(history.as_ref());
        let mut newest_owner = None;
        if let DocumentLifecycleObservation::Observed(lifecycle) = &before {
            if lifecycle.state == DocumentHistoryState::Deleted {
                let newest_revision = lifecycle.remaining_revisions;
                newest_owner = history.as_ref().and_then(|history| {
                    history
                        .entries
                        .iter()
                        .find(|entry| entry.revision == newest_revision)
                        .map(|entry| entry.document.owner_id())
                });
                if newest_owner.is_none() && newest_revision != 0 {
                    let newest_query = DocumentHistoryQuery {
                        filter: DocumentHistoryFilter::Revision(newest_revision),
                        ..query.clone()
                    };
                    newest_owner = self
                        .read_erase_history(newest_query)
                        .await
                        .and_then(|page| {
                            if page.lifecycle.as_ref() != Some(lifecycle) {
                                return None;
                            }
                            page.entries
                                .iter()
                                .find(|entry| entry.revision == newest_revision)
                                .map(|entry| entry.document.owner_id())
                        });
                }
            }
        }
        if let DocumentEraseDecision::NotSubmitted(reason) =
            decide(&before, *submitting_identity_id, newest_owner)
        {
            return Ok(DocumentEraseOutcome::NotSubmitted { reason, before });
        }
        let identity = {
            let manager = self.wallet_manager.read().await;
            let info = manager
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            info.identity_manager
                .identity(submitting_identity_id)
                .map(|managed| managed.identity.clone())
                .ok_or(PlatformWalletError::IdentityNotFound(
                    *submitting_identity_id,
                ))?
        };
        // Signer callbacks may re-enter the wallet; the manager lock is released.
        let key = select_erase_key(&identity, &document_type, *contract_id, signer)?;
        submit(contract, key).await.map_err(|error| {
            preserve_signer_key_unavailable_or(error, |error| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to erase document: {error}"
                ))
            })
        })?;
        let after = observation(self.read_erase_history(query).await.as_ref());
        Ok(DocumentEraseOutcome::Submitted { before, after })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SIGNER_KEY_UNAVAILABLE_PREFIX;
    use crate::wallet::persister::NoPlatformPersistence;
    use dash_sdk::drive::query::document_history_drive_query::DocumentHistoryLifecycleTimes;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::version::PlatformVersion;

    fn observed(state: DocumentHistoryState) -> DocumentLifecycleObservation {
        let remaining_revisions = if state == DocumentHistoryState::Absent {
            0
        } else {
            3
        };
        DocumentLifecycleObservation::Observed(DocumentHistoryLifecycle {
            state,
            remaining_revisions,
            times: DocumentHistoryLifecycleTimes::default(),
        })
    }

    #[test]
    fn should_never_pay_without_an_eligible_preflight() {
        let submitter = Identifier::from([1; 32]);
        let other = Identifier::from([2; 32]);
        for (before, owner, expected) in [
            (
                observed(DocumentHistoryState::Active),
                None,
                DocumentEraseDecision::NotSubmitted(DocumentEraseNotSubmittedReason::Active),
            ),
            (
                observed(DocumentHistoryState::Absent),
                None,
                DocumentEraseDecision::NotSubmitted(DocumentEraseNotSubmittedReason::Absent),
            ),
            (
                DocumentLifecycleObservation::Unavailable,
                None,
                DocumentEraseDecision::NotSubmitted(
                    DocumentEraseNotSubmittedReason::PreflightUnavailable,
                ),
            ),
            (
                observed(DocumentHistoryState::Deleted),
                None,
                DocumentEraseDecision::NotSubmitted(
                    DocumentEraseNotSubmittedReason::PreflightUnavailable,
                ),
            ),
            (
                observed(DocumentHistoryState::Deleted),
                Some(other),
                DocumentEraseDecision::NotSubmitted(
                    DocumentEraseNotSubmittedReason::NotNewestOwner { owner: other },
                ),
            ),
            (
                observed(DocumentHistoryState::Deleted),
                Some(submitter),
                DocumentEraseDecision::Submit,
            ),
            (
                observed(DocumentHistoryState::Erasing),
                Some(other),
                DocumentEraseDecision::Submit,
            ),
            (
                observed(DocumentHistoryState::Erasing),
                None,
                DocumentEraseDecision::Submit,
            ),
        ] {
            assert_eq!(decide(&before, submitter, owner), expected);
        }
    }

    #[test]
    fn should_release_the_in_flight_guard_on_drop() {
        let gate = DocumentEraseGate::default();
        let contract = Identifier::from([1; 32]);
        let document = Identifier::from([2; 32]);
        let guard = gate.enter(contract, document).unwrap();
        assert!(matches!(
            gate.enter(contract, document),
            Err(PlatformWalletError::DocumentEraseBusy)
        ));
        assert!(gate.enter(contract, Identifier::from([3; 32])).is_ok());
        drop(guard);
        assert!(gate.enter(contract, document).is_ok());
    }
    fn key(id: u32, level: dpp::identity::SecurityLevel) -> IdentityPublicKey {
        dpp::identity::identity_public_key::v0::IdentityPublicKeyV0 {
            id,
            purpose: dpp::identity::Purpose::AUTHENTICATION,
            security_level: level,
            key_type: KeyType::ECDSA_SECP256K1,
            data: vec![id as u8; 33].into(),
            ..Default::default()
        }
        .into()
    }

    fn bounds(
        key: &mut IdentityPublicKey,
        bounds: Option<dpp::identity::identity_public_key::contract_bounds::ContractBounds>,
    ) {
        if let IdentityPublicKey::V0(key) = key {
            key.contract_bounds = bounds;
        }
    }

    fn contract() -> DataContract {
        contract_with_requirement(dpp::identity::SecurityLevel::HIGH)
    }

    fn contract_with_requirement(requirement: dpp::identity::SecurityLevel) -> DataContract {
        use dpp::data_contract::factory::DataContractFactory;
        use dpp::platform_value::platform_value;
        DataContractFactory::new(PlatformVersion::latest().protocol_version)
            .unwrap()
            .create_with_value_config(
                Identifier::from([1; 32]),
                1,
                platform_value!({
                    "note": { "type": "object", "documentsKeepHistory": true,
                        "canBeErased": true, "signatureSecurityLevelRequirement": requirement as u8,
                    "properties": {"message": {"type": "string", "maxLength": 64, "position": 0}}, "additionalProperties": false }
                }),
                None,
                None,
            )
            .unwrap()
            .data_contract_owned()
    }

    #[test]
    fn should_select_only_usable_erase_keys_for_the_real_type() {
        use super::super::signing_key::tests::available;
        use dpp::identity::accessors::IdentitySettersV0;
        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeySettersV0;
        use dpp::identity::identity_public_key::contract_bounds::ContractBounds;
        use dpp::identity::{Identity, SecurityLevel};
        let contract = contract();
        let document_type = contract.document_type_cloned_for_name("note").unwrap();
        let mut identity = Identity::default_versioned(PlatformVersion::latest()).unwrap();
        let mut wrong_contract = key(1, SecurityLevel::CRITICAL);
        bounds(
            &mut wrong_contract,
            Some(ContractBounds::SingleContract {
                id: Identifier::from([9; 32]),
            }),
        );
        let mut wrong_type = key(2, SecurityLevel::CRITICAL);
        bounds(
            &mut wrong_type,
            Some(ContractBounds::SingleContractDocumentType {
                id: contract.id(),
                document_type_name: "other".into(),
            }),
        );
        let mut group = key(3, SecurityLevel::CRITICAL);
        bounds(
            &mut group,
            Some(ContractBounds::ContractGroup { id: contract.id() }),
        );
        let mut disabled = key(5, SecurityLevel::CRITICAL);
        disabled.set_disabled_at(1);
        for candidate in [
            key(0, SecurityLevel::MASTER),
            wrong_contract,
            wrong_type,
            group,
            key(4, SecurityLevel::CRITICAL).with_limits(None, Some(1)),
            disabled,
            key(6, SecurityLevel::HIGH).with_limits(Some(100), None),
            key(7, SecurityLevel::HIGH),
            key(8, SecurityLevel::CRITICAL),
        ] {
            identity.add_public_key(candidate);
        }
        let signer = available(&[0, 1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(
            select_erase_key(&identity, &document_type, contract.id(), &signer)
                .unwrap()
                .id(),
            7
        );
        let error = select_erase_key(
            &identity,
            &document_type,
            contract.id(),
            &available(&[0, 1, 2, 3, 4, 5]),
        )
        .unwrap_err();
        assert!(error.to_string().contains(SIGNER_KEY_UNAVAILABLE_PREFIX));
        identity.set_public_keys(Default::default());
        identity.add_public_key(key(0, SecurityLevel::MASTER));
        assert!(matches!(
            select_erase_key(&identity, &document_type, contract.id(), &signer),
            Err(PlatformWalletError::InvalidIdentityData(_))
        ));
    }

    #[derive(Default)]
    struct Contracts(std::sync::Mutex<Option<Arc<DataContract>>>);

    impl dash_sdk::platform::ContextProvider for Contracts {
        fn get_data_contract(
            &self,
            id: &Identifier,
            _: &dpp::version::PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, dash_sdk::error::ContextProviderError> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .as_ref()
                .filter(|c| c.id() == *id)
                .cloned())
        }
        fn register_data_contract(&self, contract: Arc<DataContract>) {
            *self.0.lock().unwrap() = Some(contract);
        }
        fn get_token_configuration(
            &self,
            _: &Identifier,
        ) -> Result<
            Option<dpp::data_contract::TokenConfiguration>,
            dash_sdk::error::ContextProviderError,
        > {
            Ok(None)
        }
        fn get_quorum_public_key(
            &self,
            _: u32,
            _: [u8; 32],
            _: u32,
        ) -> Result<[u8; 48], dash_sdk::error::ContextProviderError> {
            Err(dash_sdk::error::ContextProviderError::Config(
                "no test quorum".into(),
            ))
        }
        fn get_platform_activation_height(
            &self,
        ) -> Result<u32, dash_sdk::error::ContextProviderError> {
            Ok(1)
        }
    }

    fn history_query(
        contract: &DataContract,
        filter: DocumentHistoryFilter,
    ) -> DocumentHistoryQuery {
        DocumentHistoryQuery {
            data_contract_id: contract.id(),
            document_type_name: "note".into(),
            document_id: Identifier::from([2; 32]),
            filter,
            limit: Some(1),
        }
    }

    fn history(state: DocumentHistoryState, owner: Identifier, newest: bool) -> DocumentHistory {
        let absent = state == DocumentHistoryState::Absent;
        DocumentHistory {
            lifecycle: match observed(state) {
                DocumentLifecycleObservation::Observed(lifecycle) => Some(lifecycle),
                _ => unreachable!(),
            },
            entries: if absent {
                vec![]
            } else {
                vec![dash_sdk::query_types::DocumentHistoryEntry {
                    time_ms: 1,
                    revision: if newest { 3 } else { 1 },
                    document: dpp::document::DocumentV0 {
                        id: Identifier::from([2; 32]),
                        owner_id: owner,
                        revision: Some(if newest { 3 } else { 1 }),
                        ..Default::default()
                    }
                    .into(),
                }]
            },
        }
    }

    #[tokio::test]
    async fn should_register_the_contract_before_skipping_an_active_document() {
        use super::super::signing_key::tests::{available, wallet_with_signing_keys_on};
        let contract = contract();
        let provider = Arc::new(Contracts::default());
        let mut sdk = dash_sdk::SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .with_context_provider_arc(provider.clone())
            .build()
            .unwrap();
        sdk.mock()
            .expect_fetch(contract.id(), Some(contract.clone()))
            .await
            .unwrap();
        sdk.mock()
            .expect_fetch(
                history_query(&contract, DocumentHistoryFilter::StartAtTime(0)),
                Some(history(
                    DocumentHistoryState::Active,
                    Identifier::default(),
                    true,
                )),
            )
            .await
            .unwrap();
        let wallet = wallet_with_signing_keys_on(sdk, Arc::new(NoPlatformPersistence)).await;
        assert!(provider.0.lock().unwrap().is_none());
        let outcome = wallet
            .identity()
            .erase_and_observe_with_submit(
                &Identifier::default(),
                &contract.id(),
                "note",
                &Identifier::from([2; 32]),
                &available(&[]),
                |_, _| async { panic!("ACTIVE must not submit") },
            )
            .await
            .unwrap();
        assert!(matches!(
            outcome,
            DocumentEraseOutcome::NotSubmitted {
                reason: DocumentEraseNotSubmittedReason::Active,
                ..
            }
        ));
        assert_eq!(
            provider.0.lock().unwrap().as_ref().unwrap().id(),
            contract.id()
        );
    }

    #[tokio::test]
    async fn should_observe_once_after_submission_and_keep_closing_failure_separate() {
        use super::super::signing_key::tests::{available, wallet_with_signing_keys_on};
        for closing_state in [
            Some(DocumentHistoryState::Active),
            Some(DocumentHistoryState::Absent),
            Some(DocumentHistoryState::Deleted),
            Some(DocumentHistoryState::Erasing),
            None,
        ] {
            let contract = contract();
            let mut sdk = dash_sdk::SdkBuilder::new_mock()
                .with_version(PlatformVersion::latest())
                .build()
                .unwrap();
            let query = history_query(&contract, DocumentHistoryFilter::StartAtTime(0));
            sdk.mock()
                .expect_fetch(contract.id(), Some(contract.clone()))
                .await
                .unwrap();
            sdk.mock()
                .expect_fetch(
                    query.clone(),
                    Some(history(
                        DocumentHistoryState::Erasing,
                        Identifier::from([9; 32]),
                        true,
                    )),
                )
                .await
                .unwrap();
            let mut control = sdk.clone();
            let wallet = wallet_with_signing_keys_on(sdk, Arc::new(NoPlatformPersistence)).await;
            let calls = std::sync::atomic::AtomicUsize::new(0);
            let outcome = wallet
                .identity()
                .erase_and_observe_with_submit(
                    &Identifier::default(),
                    &contract.id(),
                    "note",
                    &Identifier::from([2; 32]),
                    &available(&[1]),
                    |_, _| async {
                        calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        control
                            .mock()
                            .remove_fetch_expectation::<DocumentHistory, _>(query.clone())
                            .await;
                        if let Some(state) = closing_state {
                            control
                                .mock()
                                .expect_fetch(
                                    query,
                                    Some(history(state, Identifier::from([9; 32]), true)),
                                )
                                .await
                                .unwrap();
                        }
                        Ok(())
                    },
                )
                .await
                .unwrap();
            assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
            assert_eq!(
                outcome,
                DocumentEraseOutcome::Submitted {
                    before: observed(DocumentHistoryState::Erasing),
                    after: closing_state
                        .map(observed)
                        .unwrap_or(DocumentLifecycleObservation::Unavailable),
                }
            );
        }
    }

    #[tokio::test]
    async fn should_check_the_newest_deleted_owner_and_fail_closed_on_unread_preflight() {
        use super::super::signing_key::tests::{available, wallet_with_signing_keys_on};
        let other = Identifier::from([9; 32]);
        for (state, metadata, newest, owner, owner_read, expected) in [
            (
                DocumentHistoryState::Absent,
                true,
                true,
                other,
                true,
                DocumentEraseNotSubmittedReason::Absent,
            ),
            (
                DocumentHistoryState::Deleted,
                false,
                true,
                other,
                true,
                DocumentEraseNotSubmittedReason::PreflightUnavailable,
            ),
            (
                DocumentHistoryState::Deleted,
                true,
                true,
                other,
                true,
                DocumentEraseNotSubmittedReason::NotNewestOwner { owner: other },
            ),
            (
                DocumentHistoryState::Deleted,
                true,
                false,
                other,
                true,
                DocumentEraseNotSubmittedReason::NotNewestOwner { owner: other },
            ),
            (
                DocumentHistoryState::Deleted,
                true,
                false,
                other,
                false,
                DocumentEraseNotSubmittedReason::PreflightUnavailable,
            ),
        ] {
            let contract = contract();
            let mut sdk = dash_sdk::SdkBuilder::new_mock()
                .with_version(PlatformVersion::latest())
                .build()
                .unwrap();
            sdk.mock()
                .expect_fetch(contract.id(), Some(contract.clone()))
                .await
                .unwrap();
            let mut page = history(state, owner, newest);
            if !metadata {
                page.lifecycle = None;
            }
            sdk.mock()
                .expect_fetch(
                    history_query(&contract, DocumentHistoryFilter::StartAtTime(0)),
                    Some(page),
                )
                .await
                .unwrap();
            if !newest && owner_read {
                sdk.mock()
                    .expect_fetch(
                        history_query(&contract, DocumentHistoryFilter::Revision(3)),
                        Some(history(state, owner, true)),
                    )
                    .await
                    .unwrap();
            }
            let wallet = wallet_with_signing_keys_on(sdk, Arc::new(NoPlatformPersistence)).await;
            let outcome = wallet
                .identity()
                .erase_and_observe_with_submit(
                    &Identifier::default(),
                    &contract.id(),
                    "note",
                    &Identifier::from([2; 32]),
                    &available(&[]),
                    |_, _| async { panic!("ineligible preflight must not submit") },
                )
                .await
                .unwrap();
            assert!(
                matches!(outcome, DocumentEraseOutcome::NotSubmitted { reason, .. } if reason == expected)
            );
        }
    }

    #[test]
    fn should_skip_each_ineligible_key_even_when_the_signer_has_it() {
        use super::super::signing_key::tests::{available, identity};
        use dpp::identity::identity_public_key::contract_bounds::ContractBounds;
        use dpp::identity::{Purpose, SecurityLevel};
        let contract = contract();
        let document_type = contract.document_type_cloned_for_name("note").unwrap();
        let mut wrong_contract = key(1, SecurityLevel::HIGH);
        bounds(
            &mut wrong_contract,
            Some(ContractBounds::SingleContract {
                id: Identifier::from([9; 32]),
            }),
        );
        let mut wrong_type = key(1, SecurityLevel::HIGH);
        bounds(
            &mut wrong_type,
            Some(ContractBounds::SingleContractDocumentType {
                id: contract.id(),
                document_type_name: "other".into(),
            }),
        );
        let mut group = key(1, SecurityLevel::HIGH);
        bounds(
            &mut group,
            Some(ContractBounds::ContractGroup { id: contract.id() }),
        );
        for candidate in [
            wrong_contract,
            wrong_type,
            group,
            key(1, SecurityLevel::HIGH).with_limits(None, Some(1)),
            key(1, SecurityLevel::MASTER),
        ] {
            let mut subject = identity(
                Purpose::AUTHENTICATION,
                SecurityLevel::HIGH,
                KeyType::ECDSA_SECP256K1,
            );
            subject.add_public_key(candidate);
            assert_eq!(
                select_erase_key(&subject, &document_type, contract.id(), &available(&[1, 2]))
                    .unwrap()
                    .id(),
                2
            );
        }
    }

    #[test]
    fn should_refuse_a_stored_contested_type_even_when_it_allows_erasure() {
        use dpp::data_contract::config::DataContractConfig;
        use dpp::platform_value::platform_value;
        let version = PlatformVersion::latest();
        let config = DataContractConfig::default_for_version(version).unwrap();
        let document_type = DocumentType::try_from_schema(
            Identifier::from([1; 32]),
            1,
            config.version(),
            "note",
            platform_value!({
                "type": "object", "documentsKeepHistory": true,
                "documentsMutable": false, "canBeDeleted": true, "canBeErased": true,
                "indices": [{
                    "name": "byMessage", "properties": [{"message": "asc"}], "unique": true,
                    "contested": {
                        "fieldMatches": [{"field": "message", "regexPattern": "^[a-z]{3,10}$"}],
                        "resolution": 0
                    }
                }],
                "properties": {"message": {"type": "string", "maxLength": 63, "position": 0}},
                "required": ["message"], "additionalProperties": false
            }),
            None,
            &Default::default(),
            &config,
            false,
            &mut vec![],
            version,
        )
        .unwrap();
        assert!(document_type.documents_keep_history());
        assert!(document_type.documents_can_be_erased());
        assert!(document_type.find_contested_index().is_some());
        assert!(validate_erasable_type(&document_type).is_err());
    }

    #[tokio::test]
    async fn should_refuse_contract_types_before_reading_or_signing() {
        use super::super::signing_key::tests::{available, wallet_with_signing_keys_on};
        use dpp::data_contract::factory::DataContractFactory;
        use dpp::platform_value::platform_value;
        for keeps_history in [false, true] {
            let contract = DataContractFactory::new(
                PlatformVersion::latest().protocol_version,
            )
            .unwrap()
            .create_with_value_config(
                Identifier::from([1; 32]),
                1,
                platform_value!({
                    "note": { "type": "object", "documentsKeepHistory": keeps_history,
                        "canBeErased": false, "properties": {"message": {"type": "string", "maxLength": 64, "position": 0}}, "additionalProperties": false }
                }),
                None,
                None,
            )
            .unwrap()
            .data_contract_owned();
            let mut sdk = dash_sdk::SdkBuilder::new_mock()
                .with_version(PlatformVersion::latest())
                .build()
                .unwrap();
            sdk.mock()
                .expect_fetch(contract.id(), Some(contract.clone()))
                .await
                .unwrap();
            let wallet = wallet_with_signing_keys_on(sdk, Arc::new(NoPlatformPersistence)).await;
            let result = wallet
                .identity()
                .erase_and_observe_with_submit(
                    &Identifier::default(),
                    &contract.id(),
                    "note",
                    &Identifier::from([2; 32]),
                    &available(&[]),
                    |_, _| async { panic!("contract refusal must not submit") },
                )
                .await;
            assert!(matches!(
                result,
                Err(PlatformWalletError::InvalidIdentityData(_))
            ));
        }
    }

    #[tokio::test]
    async fn should_submit_deleted_only_for_the_newest_owner_and_release_lock_before_signer() {
        use super::super::signing_key::tests::{
            available, lock_checking_signer, wallet_with_signing_keys_on,
        };
        let contract = contract();
        for no_key in [false, true] {
            let mut sdk = dash_sdk::SdkBuilder::new_mock()
                .with_version(PlatformVersion::latest())
                .build()
                .unwrap();
            sdk.mock()
                .expect_fetch(contract.id(), Some(contract.clone()))
                .await
                .unwrap();
            sdk.mock()
                .expect_fetch(
                    history_query(&contract, DocumentHistoryFilter::StartAtTime(0)),
                    Some(history(
                        DocumentHistoryState::Deleted,
                        Identifier::from([9; 32]),
                        false,
                    )),
                )
                .await
                .unwrap();
            sdk.mock()
                .expect_fetch(
                    history_query(&contract, DocumentHistoryFilter::Revision(3)),
                    Some(history(
                        DocumentHistoryState::Deleted,
                        Identifier::default(),
                        true,
                    )),
                )
                .await
                .unwrap();
            let wallet = wallet_with_signing_keys_on(sdk, Arc::new(NoPlatformPersistence)).await;
            if no_key {
                let signer = lock_checking_signer(wallet.identity());
                let error = wallet
                    .identity()
                    .erase_and_observe_with_submit(
                        &Identifier::default(),
                        &contract.id(),
                        "note",
                        &Identifier::from([2; 32]),
                        &signer,
                        |_, _| async { panic!("unavailable key must not submit") },
                    )
                    .await
                    .unwrap_err();
                assert!(error.to_string().contains(SIGNER_KEY_UNAVAILABLE_PREFIX));
            } else {
                let outcome = wallet
                    .identity()
                    .erase_and_observe_with_submit(
                        &Identifier::default(),
                        &contract.id(),
                        "note",
                        &Identifier::from([2; 32]),
                        &available(&[1]),
                        |_, _| async { Ok(()) },
                    )
                    .await
                    .unwrap();
                assert!(matches!(outcome, DocumentEraseOutcome::Submitted { .. }));
            }
        }
    }

    #[tokio::test]
    async fn should_not_submit_when_the_preflight_read_fails() {
        use super::super::signing_key::tests::{available, wallet_with_signing_keys_on};
        let contract = contract();
        let mut sdk = dash_sdk::SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .build()
            .unwrap();
        sdk.mock()
            .expect_fetch(contract.id(), Some(contract.clone()))
            .await
            .unwrap();
        let wallet = wallet_with_signing_keys_on(sdk, Arc::new(NoPlatformPersistence)).await;
        let outcome = wallet
            .identity()
            .erase_and_observe_with_submit(
                &Identifier::default(),
                &contract.id(),
                "note",
                &Identifier::from([2; 32]),
                &available(&[]),
                |_, _| async { panic!("unread preflight must not submit") },
            )
            .await
            .unwrap();
        assert!(matches!(
            outcome,
            DocumentEraseOutcome::NotSubmitted {
                reason: DocumentEraseNotSubmittedReason::PreflightUnavailable,
                before: DocumentLifecycleObservation::Unavailable
            }
        ));
    }
    #[test]
    fn should_skip_an_expired_erase_key_even_when_it_is_available() {
        use super::super::signing_key::tests::{available, identity};
        use dpp::identity::{Purpose, SecurityLevel};
        let contract = contract();
        let document_type = contract.document_type_cloned_for_name("note").unwrap();
        let mut subject = identity(
            Purpose::AUTHENTICATION,
            SecurityLevel::HIGH,
            KeyType::ECDSA_SECP256K1,
        );
        subject.add_public_key(key(1, SecurityLevel::HIGH).with_limits(None, Some(1)));
        assert_eq!(
            select_erase_key(&subject, &document_type, contract.id(), &available(&[1, 2]))
                .unwrap()
                .id(),
            2
        );
    }

    #[test]
    fn should_prefer_an_unlimited_erase_key_over_an_earlier_budgeted_key() {
        use super::super::signing_key::tests::{available, identity};
        use dpp::identity::{Purpose, SecurityLevel};
        let contract = contract();
        let document_type = contract.document_type_cloned_for_name("note").unwrap();
        let mut subject = identity(
            Purpose::AUTHENTICATION,
            SecurityLevel::HIGH,
            KeyType::ECDSA_SECP256K1,
        );
        subject.add_public_key(key(1, SecurityLevel::HIGH).with_limits(Some(100), None));
        assert_eq!(
            select_erase_key(&subject, &document_type, contract.id(), &available(&[1, 2]))
                .unwrap()
                .id(),
            2
        );
    }

    #[test]
    fn should_use_the_real_document_requirement_and_exclude_master() {
        use super::super::signing_key::tests::{available, identity};
        use dpp::identity::{Purpose, SecurityLevel};
        for (requirement, expected) in [
            (SecurityLevel::CRITICAL, 3),
            (SecurityLevel::HIGH, 2),
            (SecurityLevel::MEDIUM, 1),
        ] {
            let contract = contract_with_requirement(requirement);
            let document_type = contract.document_type_cloned_for_name("note").unwrap();
            let mut subject = identity(
                Purpose::AUTHENTICATION,
                SecurityLevel::MEDIUM,
                KeyType::ECDSA_SECP256K1,
            );
            for candidate in [
                key(0, SecurityLevel::MASTER),
                key(2, SecurityLevel::HIGH),
                key(3, SecurityLevel::CRITICAL),
            ] {
                subject.add_public_key(candidate);
            }
            assert_eq!(
                select_erase_key(
                    &subject,
                    &document_type,
                    contract.id(),
                    &available(&[0, 1, 2, 3])
                )
                .unwrap()
                .id(),
                expected
            );
        }
    }
}
