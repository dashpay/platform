//! Append entries to a private document type's store, paid from the shielded pool.

use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use crate::{Error, Sdk};
use dpp::platform_value::{BinaryData, Identifier};
use dpp::shielded::OrchardBundleParams;
use dpp::state_transition::private_document_create_from_shielded_pool_transition::methods::PrivateDocumentCreateFromShieldedPoolTransitionMethodsV0;
use dpp::state_transition::private_document_create_from_shielded_pool_transition::PrivateDocumentCreateFromShieldedPoolTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use dpp::version::feature_initial_protocol_versions::PRIVATE_DOCUMENT_CREATE_FROM_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION;

/// Append sealed entries to the store of a private document type (type 29), paying the fee
/// from shielded notes. The transition names no identity: the bundle's spends pay, and its
/// binding signature commits to the contract, the type name and every entry, so a node cannot
/// move the payment to other entries.
///
/// The bundle must be an already proven Orchard spend whose value balance is `fee` (see
/// `dpp::shielded::builder::build_private_document_create_from_shielded_pool_transition`, which
/// builds the whole transition from notes). Every entry must be exactly the type's
/// `entrySize`, as `platform_encryption::seal_private_document_entry` builds them.
#[async_trait::async_trait]
pub trait PrivateDocumentCreateFromShieldedPool {
    /// Build and structure-check the transition without broadcasting it.
    #[allow(clippy::too_many_arguments)]
    fn private_document_create_from_shielded_pool_transition(
        &self,
        data_contract_id: Identifier,
        document_type_name: String,
        entries: Vec<BinaryData>,
        fee: u64,
        bundle: OrchardBundleParams,
    ) -> Result<StateTransition, Error>;

    /// Build, broadcast, and wait for the proof of the affected state. Returns the
    /// `VerifiedShieldedNullifiers` proof result: it proves the bundle's nullifiers spent, which
    /// is when a wallet marks its notes spent. It does not prove that this transition was the
    /// spend, nor that its entries were appended: another transition spending the same notes
    /// proves the same. A wallet that needs the entries reads the store.
    #[allow(clippy::too_many_arguments)]
    async fn private_document_create_from_shielded_pool(
        &self,
        data_contract_id: Identifier,
        document_type_name: String,
        entries: Vec<BinaryData>,
        fee: u64,
        bundle: OrchardBundleParams,
        settings: Option<PutSettings>,
    ) -> Result<StateTransitionProofResult, Error>;
}

#[async_trait::async_trait]
impl PrivateDocumentCreateFromShieldedPool for Sdk {
    fn private_document_create_from_shielded_pool_transition(
        &self,
        data_contract_id: Identifier,
        document_type_name: String,
        entries: Vec<BinaryData>,
        fee: u64,
        bundle: OrchardBundleParams,
    ) -> Result<StateTransition, Error> {
        let OrchardBundleParams {
            actions,
            anchor,
            proof,
            binding_signature,
        } = bundle;

        let state_transition = PrivateDocumentCreateFromShieldedPoolTransition::try_from_bundle(
            data_contract_id,
            document_type_name,
            entries,
            actions,
            fee,
            anchor,
            proof,
            binding_signature,
            self.version(),
        )?;
        ensure_valid_state_transition_structure(&state_transition, self.version())?;

        Ok(state_transition)
    }

    async fn private_document_create_from_shielded_pool(
        &self,
        data_contract_id: Identifier,
        document_type_name: String,
        entries: Vec<BinaryData>,
        fee: u64,
        bundle: OrchardBundleParams,
        settings: Option<PutSettings>,
    ) -> Result<StateTransitionProofResult, Error> {
        // An unpinned SDK starts below protocol version 15, whose limits admit no entries: the
        // network's version is established before the transition is built against it.
        self.require_protocol_version(
            PRIVATE_DOCUMENT_CREATE_FROM_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION,
            "private documents",
        )
        .await?;
        let state_transition = self.private_document_create_from_shielded_pool_transition(
            data_contract_id,
            document_type_name,
            entries,
            fee,
            bundle,
        )?;
        let proof_result = state_transition
            .broadcast_and_wait_for_affected_state::<StateTransitionProofResult>(self, settings)
            .await?;
        Ok(proof_result)
    }
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use super::*;
    use crate::SdkBuilder;
    use dpp::shielded::SerializedAction;
    use dpp::version::PlatformVersion;

    fn bundle() -> OrchardBundleParams {
        let action = SerializedAction {
            nullifier: [1; 32],
            rk: [2; 32],
            cmx: [3; 32],
            encrypted_note: vec![4; 216],
            cv_net: [5; 32],
            spend_auth_sig: [6; 64],
        };
        OrchardBundleParams {
            actions: vec![action.clone(), action],
            anchor: [7; 32],
            proof: vec![8; 64],
            binding_signature: [9; 64],
        }
    }

    /// Entries consensus refuses, here none or two the same, are refused before anything is
    /// broadcast.
    #[test]
    fn should_refuse_entries_consensus_refuses_before_broadcasting() {
        let sdk: Sdk = SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .build()
            .expect("expected a mock sdk");
        for (entries, refusal) in [
            (vec![], "carries 0 entries"),
            (
                vec![BinaryData::new(vec![1; 589]), BinaryData::new(vec![1; 589])],
                "is the same as entry 0",
            ),
        ] {
            let result = sdk.private_document_create_from_shielded_pool_transition(
                Identifier::new([4; 32]),
                "privateContactRequest".to_string(),
                entries,
                114_140_000,
                bundle(),
            );
            let error = result.expect_err("the entries are refused").to_string();
            assert!(error.contains(refusal), "{refusal}: {error}");
        }
    }

    /// Below protocol version 15 the send is refused before the transition is built, rather
    /// than built against limits that admit no entries.
    #[tokio::test]
    async fn should_refuse_to_send_below_protocol_version_15() {
        let sdk: Sdk = SdkBuilder::new_mock()
            .with_version(PlatformVersion::get(14).expect("protocol version 14"))
            .build()
            .expect("expected a mock sdk");
        let error = sdk
            .private_document_create_from_shielded_pool(
                Identifier::new([4; 32]),
                "privateContactRequest".to_string(),
                vec![BinaryData::new(vec![1; 589])],
                114_140_000,
                bundle(),
                None,
            )
            .await
            .expect_err("protocol version 14")
            .to_string();
        assert!(error.contains("exists from protocol version 15"), "{error}");
    }
}
