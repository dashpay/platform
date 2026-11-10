//! Append entries to a private document type's store, paid from the shielded pool.

use super::broadcast::{private_document_create_requirement, BroadcastStateTransition};
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
    /// Build and structure-check the transition without broadcasting it, at the SDK's protocol
    /// version. Below protocol version 15 it is refused, naming the SDK's version: this cannot
    /// ask the network, so an unpinned SDK still at an earlier version is refreshed first
    /// ([`Sdk::refresh_protocol_version`]), as the send does.
    #[allow(clippy::too_many_arguments)]
    fn private_document_create_from_shielded_pool_transition(
        &self,
        data_contract_id: Identifier,
        document_type_name: String,
        entries: Vec<BinaryData>,
        fee: u64,
        bundle: OrchardBundleParams,
    ) -> Result<StateTransition, Error>;

    /// Build, broadcast, and wait for the proof of the affected state, once a verified answer
    /// shows the network runs protocol version 15: an SDK seeded or pinned there may face a
    /// network that does not, which would reject the transition, so the send is refused with
    /// [`Error::ProtocolVersionNotActive`] instead, before it is built. Returns the
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
        // Below protocol version 15 the limits admit no entries, so every entry would be
        // refused as one too many. The send learns the network's version first; this builder
        // cannot, so it says which version the SDK runs.
        let protocol_version = self.version().protocol_version;
        if protocol_version < PRIVATE_DOCUMENT_CREATE_FROM_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION {
            return Err(Error::Generic(format!(
                "private documents exist from protocol version \
                 {PRIVATE_DOCUMENT_CREATE_FROM_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION}; the SDK \
                 runs protocol version {protocol_version}: refresh its protocol version \
                 (Sdk::refresh_protocol_version) before building the transition, unless it is \
                 pinned below that"
            )));
        }
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
        // The network's version is established before the transition is built against it, as
        // limits below protocol version 15 admit no entries, under the caller's request settings
        // (`Sdk::establish_protocol_requirement`). The transition is sent only once a verified
        // answer shows the network serves it and this SDK speaks it, as a read of the store is
        // (the broadcast checks again, for a transition built elsewhere); otherwise the send is
        // refused with the structured activation error, the probe's failure as its cause when it
        // failed, so a caller can tell a refusal that may lift at once from one the network's
        // version or a pin explains.
        self.establish_protocol_requirement(
            private_document_create_requirement(),
            settings.map(|settings| settings.request_settings),
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
    use crate::platform::transition::broadcast::BROADCAST_STATE_TRANSITION_METHOD;
    use crate::sdk::test::expect_epoch_refresh;
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
    /// than built against limits that admit no entries, with the structured activation error:
    /// for an SDK pinned there, which asks nothing, and for one seeded there whose refresh
    /// failed, here for want of an answer: that failure is the refusal's cause, so the refusal
    /// reads as one that may lift, and the pin and the network's version read as before.
    #[tokio::test]
    async fn should_refuse_to_send_below_protocol_version_15() {
        let version_14 = PlatformVersion::get(14).expect("protocol version 14");
        for (builder, pinned) in [
            (SdkBuilder::new_mock().with_version(version_14), true),
            (
                SdkBuilder::new_mock().with_initial_version(version_14),
                false,
            ),
        ] {
            let sdk: Sdk = builder.build().expect("expected a mock sdk");
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
                .expect_err("protocol version 14");
            assert!(
                matches!(
                    &error,
                    Error::ProtocolVersionNotActive {
                        method: BROADCAST_STATE_TRANSITION_METHOD,
                        required: 15,
                        // No answer was verified.
                        network: 0,
                        pinned: refused_pinned,
                        cause,
                        ..
                    } if *refused_pinned == pinned && cause.is_some() == !pinned
                ),
                "pinned {pinned}: {error:?}"
            );
            assert_eq!(
                error.is_transient_protocol_version_refusal(),
                !pinned,
                "pinned {pinned}: {error:?}"
            );
        }
    }

    /// The builder, which cannot ask the network, refuses below protocol version 15 naming
    /// the SDK's version and the refresh that lifts it, whether the SDK is seeded or pinned
    /// there.
    #[test]
    fn should_name_the_sdk_s_version_when_building_below_protocol_version_15() {
        let version_14 = PlatformVersion::get(14).expect("protocol version 14");
        for sdk in [
            SdkBuilder::new_mock().with_initial_version(version_14),
            SdkBuilder::new_mock().with_version(version_14),
        ] {
            let sdk: Sdk = sdk.build().expect("expected a mock sdk");
            let error = sdk
                .private_document_create_from_shielded_pool_transition(
                    Identifier::new([4; 32]),
                    "privateContactRequest".to_string(),
                    vec![BinaryData::new(vec![1; 589])],
                    114_140_000,
                    bundle(),
                )
                .expect_err("protocol version 14")
                .to_string();
            assert!(
                error.contains("the SDK runs protocol version 14")
                    && error.contains("refresh_protocol_version"),
                "{error}"
            );
        }
    }

    /// An unpinned SDK seeded below protocol version 15 learns the network's version before it
    /// builds the send, and builds it against that version.
    #[tokio::test]
    async fn should_learn_the_network_version_before_building_a_send() {
        let mut sdk: Sdk = SdkBuilder::new_mock()
            .with_initial_version(PlatformVersion::get(14).expect("protocol version 14"))
            .build()
            .expect("expected a mock sdk");
        // The mock network runs the latest version
        expect_epoch_refresh(&mut sdk).await;
        let entries = vec![BinaryData::new(vec![1; 589])];

        let error = sdk
            .private_document_create_from_shielded_pool(
                Identifier::new([4; 32]),
                "privateContactRequest".to_string(),
                entries.clone(),
                114_140_000,
                bundle(),
                None,
            )
            .await
            .expect_err("the mock takes no broadcast")
            .to_string();

        assert_eq!(
            sdk.protocol_version_number(),
            PlatformVersion::latest().protocol_version
        );
        assert!(!error.contains("exists from protocol version"), "{error}");
        let built = sdk
            .private_document_create_from_shielded_pool_transition(
                Identifier::new([4; 32]),
                "privateContactRequest".to_string(),
                entries,
                114_140_000,
                bundle(),
            )
            .expect("the send builds at the learned version");
        assert!(
            matches!(
                built,
                StateTransition::PrivateDocumentCreateFromShieldedPool(_)
            ),
            "{built:?}"
        );
    }
}
