//! Decision proposal create and edit on `IdentityWallet` (protocol
//! version 15).
//!
//! rs-sdk decides what a proposal is (`dash_sdk::platform::decision_proposals`):
//! the properties a new proposal is created with, the decision proposal
//! fund its create states, refusing an edit that changes nothing, and the
//! revision an edit moves to. These operations orchestrate: they resolve
//! the contract, read the proposal, select the signing key, put the
//! document and keep the owner's proven balance, through the steps the
//! generic document create in `document.rs` takes.
//!
//! An edit reads the proposal once and puts that document at the revision
//! after the one it read. Nothing reads the proposal again before the put,
//! so when another edit lands in between, the stored revision has already
//! moved past the one this edit was built from and Platform refuses the
//! replace (`InvalidDocumentRevisionError`, 40106) instead of letting it
//! overwrite text it never saw. The refusal reaches the caller as
//! [`PlatformWalletError::Sdk`], which keeps Platform's consensus error.

use dash_sdk::platform::decision_proposals::{
    build_decision_proposal_edit, DecisionProposalDocument, DecisionProposalEdit,
    DecisionProposalInput,
};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::DocumentType;
use dpp::decision_proposal::{DECISION_PROPOSALS_CONTRACT_ID, PROPOSAL_DOCUMENT_TYPE_NAME};
use dpp::document::{Document, DocumentV0Getters};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::Identifier;

use crate::error::PlatformWalletError;

use super::*;

impl IdentityWallet {
    /// Create a decision proposal of `owner_identity_id` asking what
    /// `input` says, and return it as Platform stored it.
    ///
    /// rs-sdk builds the proposal (`Sdk::build_decision_proposal`); its
    /// create states the decision proposal fund (0.5 Dash), which the
    /// owner pays on top of the fees. The transition is signed with the
    /// owner's first AUTHENTICATION + ECDSA key at a security level the
    /// `proposal` type accepts that `signer` can sign with, and the
    /// owner's balance the proof carries is kept. Platform refuses a rule
    /// outside the contract's limits and an end date less than a week or
    /// more than four weeks after the block that creates the proposal.
    /// The proposal's id is the id of the returned document.
    ///
    /// A network below protocol version 15 has no decision proposals
    /// contract, which is an error. SDK and Platform failures, a refusal
    /// included, arrive as [`PlatformWalletError::Sdk`].
    pub async fn create_decision_proposal_with_signer<S>(
        &self,
        owner_identity_id: &Identifier,
        input: &DecisionProposalInput,
        signer: &S,
    ) -> Result<Document, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let document_type = self.decision_proposal_document_type().await?;
        let DecisionProposalDocument { document, entropy } = self
            .sdk
            .build_decision_proposal(*owner_identity_id, input)
            .await?;
        let signing_key = self
            .select_document_signing_key(owner_identity_id, &document_type, signer)
            .await?;
        Ok(self
            .put_document_keeping_owner_balance(
                &document,
                document_type,
                Some(entropy.to_buffer()),
                signing_key,
                signer,
                None,
            )
            .await?)
    }

    /// Give the decision proposal `proposal_id` of `owner_identity_id`
    /// the new text `edit` sets, and return it as Platform stored it.
    ///
    /// Reads the proposal once, applies the edit with rs-sdk's
    /// `build_decision_proposal_edit` (which refuses an edit that changes
    /// nothing and moves to the revision after the one read) and puts
    /// that document as a replace, signed and kept like
    /// [`Self::create_decision_proposal_with_signer`]. The put carries the
    /// revision after the one read: if another edit lands in between,
    /// Platform refuses this one (`InvalidDocumentRevisionError`, 40106)
    /// rather than overwrite it, and the caller reads the proposal again.
    /// Platform also refuses an edit more than a day after the proposal
    /// was created (or a fifth of the time it runs, if that is shorter).
    ///
    /// # Errors
    /// * [`PlatformWalletError::DecisionProposalNotFound`] when there is
    ///   no such proposal;
    /// * [`PlatformWalletError::InvalidParameter`] when the proposal
    ///   belongs to another identity, or the edit changes nothing;
    /// * [`PlatformWalletError::Sdk`] for SDK and Platform failures,
    ///   Platform's refusals included.
    pub async fn edit_decision_proposal_with_signer<S>(
        &self,
        owner_identity_id: &Identifier,
        proposal_id: &Identifier,
        edit: &DecisionProposalEdit,
        signer: &S,
    ) -> Result<Document, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let document_type = self.decision_proposal_document_type().await?;
        let read = self.sdk.fetch_decision_proposal(*proposal_id).await?;
        let replacement =
            decision_proposal_replacement(owner_identity_id, proposal_id, read, edit)?;
        let signing_key = self
            .select_document_signing_key(owner_identity_id, &document_type, signer)
            .await?;
        Ok(self
            .put_document_keeping_owner_balance(
                &replacement,
                document_type,
                None,
                signing_key,
                signer,
                None,
            )
            .await?)
    }

    /// The `proposal` document type of the decision proposals contract,
    /// which rs-sdk resolves (refusing a network below protocol version
    /// 15). The contract is registered for the proof check that follows
    /// the put.
    async fn decision_proposal_document_type(&self) -> Result<DocumentType, PlatformWalletError> {
        let contract = self.sdk.fetch_decision_proposals_contract().await?;
        self.register_contract_for_proof_verification(&contract);
        contract
            .document_type_cloned_for_name(PROPOSAL_DOCUMENT_TYPE_NAME)
            .map_err(|e| PlatformWalletError::Sdk(dash_sdk::Error::Protocol(e.into())))
    }
}

/// The document an edit of the proposal `proposal_id` puts: `read`, the
/// proposal as read once for this edit, with the text `edit` sets and the
/// revision after the one read. A proposal that is not there, or that
/// belongs to an identity other than `owner_identity_id`, is refused.
fn decision_proposal_replacement(
    owner_identity_id: &Identifier,
    proposal_id: &Identifier,
    read: Option<Document>,
    edit: &DecisionProposalEdit,
) -> Result<Document, PlatformWalletError> {
    let proposal = read.ok_or(PlatformWalletError::DecisionProposalNotFound {
        proposal_id: *proposal_id,
    })?;
    if proposal.owner_id() != *owner_identity_id {
        return Err(PlatformWalletError::InvalidParameter(format!(
            "decision proposal {proposal_id} belongs to identity {}, not to {owner_identity_id}",
            proposal.owner_id()
        )));
    }
    build_decision_proposal_edit(
        &DECISION_PROPOSALS_CONTRACT_ID,
        PROPOSAL_DOCUMENT_TYPE_NAME,
        &proposal,
        edit,
    )
    .map_err(|error| match error {
        // rs-sdk refuses the edit itself (one that changes nothing) with
        // its own text; anything else keeps its SDK error.
        dash_sdk::Error::Generic(message) => PlatformWalletError::InvalidParameter(message),
        other => PlatformWalletError::Sdk(other),
    })
}

#[cfg(test)]
mod tests {
    use super::super::signing_key::tests::{
        available, identity, lock_checking_signer, wallet_with_signing_keys_on,
    };
    use super::*;
    use crate::error::SIGNER_KEY_UNAVAILABLE_PREFIX;
    use crate::wallet::persister::NoPlatformPersistence;
    use async_trait::async_trait;
    use dash_sdk::dapi_grpc::platform::v0::GetDocumentsRequest;
    use dash_sdk::error::StateTransitionBroadcastError;
    use dash_sdk::platform::decision_proposals::build_decision_proposal_document;
    use dash_sdk::platform::transition::broadcast_request::BroadcastRequestForStateTransition;
    use dash_sdk::platform::{DocumentQuery, Query};
    use dash_sdk::query_types::IdentityContractNonceFetcher;
    use dpp::address_funds::AddressWitness;
    use dpp::consensus::state::document::invalid_document_revision_error::InvalidDocumentRevisionError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::data_contract::DataContract;
    use dpp::decision_proposal::properties;
    use dpp::document::{DocumentV0Setters, INITIAL_REVISION};
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::{BinaryData, Bytes32, Value};
    use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
    use dpp::state_transition::batch_transition::BatchTransition;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::version::PlatformVersion;
    use dpp::ProtocolError;
    use std::sync::Arc;

    const CREATED_AT: u64 = 1_700_000_000_000;
    /// The identity contract nonce Platform reports for the owner.
    const STORED_NONCE: u64 = 4;
    const NEW_TITLE: &str = "Raise the block size to 4 MB?";

    /// The owner identity `wallet_with_signing_keys_on` holds.
    fn owner() -> Identifier {
        Identifier::default()
    }

    fn proposals_contract() -> Arc<DataContract> {
        Arc::new(
            load_system_data_contract(
                SystemDataContract::DecisionProposals,
                PlatformVersion::latest(),
            )
            .expect("the decision proposals contract loads at the latest version"),
        )
    }

    fn input() -> DecisionProposalInput {
        DecisionProposalInput {
            title: "Raise the block size?".to_string(),
            description: "Yes doubles it; no keeps it.".to_string(),
            url: Some("https://forum.dash.org/t/block-size".to_string()),
            pass_percent: 60,
            minimum_turnout_percent: 10,
            end_date: CREATED_AT + 14 * 86_400_000,
        }
    }

    /// A proposal of `owner_id` built by rs-sdk, as Platform stores it at
    /// its first revision.
    fn stored_proposal(owner_id: Identifier) -> Document {
        let mut document = build_decision_proposal_document(
            &proposals_contract(),
            owner_id,
            &input(),
            Bytes32::new([9; 32]),
        )
        .expect("builds")
        .document;
        document.set_created_at(Some(CREATED_AT));
        document.set_updated_at(Some(CREATED_AT));
        document
    }

    fn retitle() -> DecisionProposalEdit {
        DecisionProposalEdit {
            title: Some(NEW_TITLE.to_string()),
            ..Default::default()
        }
    }

    /// Serves the bundled decision proposals contract (nothing else), the
    /// way a host's context provider does, so rs-sdk resolves it without a
    /// network fetch or a dump directory.
    struct ProposalsContractProvider(Arc<DataContract>);

    impl dash_sdk::platform::ContextProvider for ProposalsContractProvider {
        fn get_data_contract(
            &self,
            id: &Identifier,
            _platform_version: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, dash_sdk::error::ContextProviderError> {
            Ok((*id == self.0.id()).then(|| Arc::clone(&self.0)))
        }

        fn get_token_configuration(
            &self,
            _token_id: &Identifier,
        ) -> Result<
            Option<dpp::data_contract::associated_token::token_configuration::TokenConfiguration>,
            dash_sdk::error::ContextProviderError,
        > {
            Ok(None)
        }

        fn get_quorum_public_key(
            &self,
            _quorum_type: u32,
            _quorum_hash: [u8; 32],
            _core_chain_locked_height: u32,
        ) -> Result<[u8; 48], dash_sdk::error::ContextProviderError> {
            Err(dash_sdk::error::ContextProviderError::Config(
                "no quorum keys in this test".to_string(),
            ))
        }

        fn get_platform_activation_height(
            &self,
        ) -> Result<dpp::prelude::CoreBlockHeight, dash_sdk::error::ContextProviderError> {
            Ok(1)
        }
    }

    /// The query rs-sdk reads the proposal `proposal_id` with.
    fn proposal_query(proposal_id: Identifier) -> DocumentQuery {
        DocumentQuery::new(proposals_contract(), PROPOSAL_DOCUMENT_TYPE_NAME)
            .expect("the proposal type")
            .with_document_id(&proposal_id)
    }

    /// A mock SDK at the latest protocol version that resolves the
    /// decision proposals contract. A request without an expectation
    /// fails.
    fn proposals_sdk() -> dash_sdk::Sdk {
        dash_sdk::SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .with_context_provider(ProposalsContractProvider(proposals_contract()))
            .build()
            .expect("mock sdk")
    }

    /// [`proposals_sdk`] answering a read of the proposal `proposal_id`
    /// with `stored`.
    async fn sdk_storing(proposal_id: Identifier, stored: Option<Document>) -> dash_sdk::Sdk {
        let mut sdk = proposals_sdk();
        sdk.mock()
            .expect_fetch(proposal_query(proposal_id), stored)
            .await
            .expect("expects the proposal read");
        sdk
    }

    /// Times the proposal `proposal_id` was read through `sdk`.
    async fn times_read(sdk: &mut dash_sdk::Sdk, proposal_id: Identifier) -> usize {
        let request: GetDocumentsRequest = Query::<GetDocumentsRequest>::query(
            &proposal_query(proposal_id),
            &sdk.query_settings(),
        )
        .expect("encodes");
        sdk.mock().request_count(&request).await
    }

    /// Signs with a fixed signature, so the transition it signs is known.
    #[derive(Debug)]
    struct FixedSignatureSigner;

    #[async_trait]
    impl Signer<IdentityPublicKey> for FixedSignatureSigner {
        async fn sign(&self, _: &IdentityPublicKey, _: &[u8]) -> Result<BinaryData, ProtocolError> {
            Ok(BinaryData::new(vec![7; 65]))
        }

        async fn sign_create_witness(
            &self,
            _: &IdentityPublicKey,
            _: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            Err(ProtocolError::Generic(
                "a document transition has no witness".to_string(),
            ))
        }

        fn can_sign_with(&self, _: &IdentityPublicKey) -> bool {
            true
        }
    }

    #[test]
    fn should_move_the_proposal_read_to_its_next_revision_with_only_the_edited_text() {
        let read = stored_proposal(owner());
        let replacement =
            decision_proposal_replacement(&owner(), &read.id(), Some(read.clone()), &retitle())
                .expect("edits");

        assert_eq!(replacement.id(), read.id());
        assert_eq!(replacement.owner_id(), owner());
        assert_eq!(replacement.revision(), Some(INITIAL_REVISION + 1));
        assert_eq!(replacement.created_at(), Some(CREATED_AT));
        let mut expected = read.properties().clone();
        expected.insert(
            properties::TITLE.to_string(),
            Value::Text(NEW_TITLE.to_string()),
        );
        assert_eq!(replacement.properties(), &expected);
    }

    /// Another client's edit lands between this edit's read and its put.
    /// The put carries the revision after the one read and the text as
    /// read plus this edit; it is not rebuilt on top of the newer
    /// revision. Platform takes a replace of the proposal it now stores
    /// only at the revision after that one, so it refuses this put
    /// (`InvalidDocumentRevisionError`) instead of publishing the
    /// description the other edit replaced.
    #[test]
    fn should_keep_the_revision_read_when_another_edit_lands_before_the_put() {
        let read = stored_proposal(owner());
        let replacement =
            decision_proposal_replacement(&owner(), &read.id(), Some(read.clone()), &retitle())
                .expect("edits");

        let concurrent = build_decision_proposal_edit(
            &DECISION_PROPOSALS_CONTRACT_ID,
            PROPOSAL_DOCUMENT_TYPE_NAME,
            &read,
            &DecisionProposalEdit {
                description: Some("Yes doubles it.".to_string()),
                ..Default::default()
            },
        )
        .expect("the other client's edit");
        assert_eq!(concurrent.revision(), Some(INITIAL_REVISION + 1));

        assert_eq!(replacement.revision(), Some(INITIAL_REVISION + 1));
        let accepted_after_concurrent = concurrent.revision().map(|revision| revision + 1);
        assert_ne!(replacement.revision(), accepted_after_concurrent);
        assert_eq!(
            replacement.get(properties::DESCRIPTION),
            read.get(properties::DESCRIPTION)
        );
    }

    #[test]
    fn should_refuse_an_edit_that_changes_nothing() {
        let read = stored_proposal(owner());
        let error = decision_proposal_replacement(
            &owner(),
            &read.id(),
            Some(read.clone()),
            &DecisionProposalEdit {
                title: Some(input().title),
                ..Default::default()
            },
        )
        .expect_err("changes nothing");
        assert!(
            matches!(error, PlatformWalletError::InvalidParameter(ref message)
                if message.contains("changes nothing")),
            "{error:?}"
        );
    }

    /// The edit reads the proposal once, then puts exactly that document
    /// at the revision after the one read: the broadcast the put sends is
    /// the replace built from it, signed with the owner's first eligible
    /// key at the next identity contract nonce. The mock answers no
    /// broadcast, so the put fails after sending it.
    #[tokio::test]
    async fn should_put_the_proposal_read_once_at_the_revision_after_it() {
        let read = stored_proposal(owner());
        let proposal_id = read.id();
        let mut sdk = sdk_storing(proposal_id, Some(read.clone())).await;
        sdk.mock()
            .expect_fetch(
                (owner(), DECISION_PROPOSALS_CONTRACT_ID),
                Some(IdentityContractNonceFetcher(STORED_NONCE)),
            )
            .await
            .expect("expects the nonce read");
        let wallet =
            wallet_with_signing_keys_on(sdk.clone(), Arc::new(NoPlatformPersistence)).await;

        let error = wallet
            .identity()
            .edit_decision_proposal_with_signer(
                &owner(),
                &proposal_id,
                &retitle(),
                &FixedSignatureSigner,
            )
            .await
            .expect_err("the mock answers no broadcast");
        assert!(matches!(error, PlatformWalletError::Sdk(_)), "{error:?}");
        assert_eq!(times_read(&mut sdk, proposal_id).await, 1);

        // The proposal as the read returned it: the mock answers with the
        // document re-encoded, so its integer values come back as Platform
        // returns them rather than as the builder made them.
        let returned = sdk
            .fetch_decision_proposal(proposal_id)
            .await
            .expect("reads")
            .expect("the stored proposal");
        let contract = proposals_contract();
        let document_type = contract
            .document_type_for_name(PROPOSAL_DOCUMENT_TYPE_NAME)
            .expect("the proposal type");
        let mut put_document = returned;
        put_document.set(properties::TITLE, Value::Text(NEW_TITLE.to_string()));
        put_document.set_revision(Some(INITIAL_REVISION + 1));
        let signing_key = identity(
            Purpose::AUTHENTICATION,
            SecurityLevel::CRITICAL,
            KeyType::ECDSA_SECP256K1,
        )
        .public_keys()
        .get(&1)
        .expect("the owner's first key")
        .clone();
        let replace = BatchTransition::new_document_replacement_transition_from_document(
            put_document,
            document_type,
            &signing_key,
            STORED_NONCE + 1,
            0,
            None,
            &FixedSignatureSigner,
            PlatformVersion::latest(),
            None,
        )
        .await
        .expect("the replace");
        let broadcast = replace
            .broadcast_request_for_state_transition()
            .expect("serializes");
        assert!(
            sdk.mock().request_count(&broadcast).await > 0,
            "the put must broadcast the replace of the proposal as read, at revision {}",
            INITIAL_REVISION + 1
        );
    }

    #[tokio::test]
    async fn should_refuse_to_edit_a_proposal_that_does_not_exist() {
        let proposal_id = Identifier::from([4; 32]);
        let mut sdk = sdk_storing(proposal_id, None).await;
        let wallet =
            wallet_with_signing_keys_on(sdk.clone(), Arc::new(NoPlatformPersistence)).await;

        let error = wallet
            .identity()
            .edit_decision_proposal_with_signer(&owner(), &proposal_id, &retitle(), &available(&[]))
            .await
            .expect_err("no such proposal");
        assert!(
            matches!(error, PlatformWalletError::DecisionProposalNotFound { proposal_id: id }
                if id == proposal_id),
            "{error:?}"
        );
        assert_eq!(times_read(&mut sdk, proposal_id).await, 1);
    }

    #[tokio::test]
    async fn should_refuse_to_edit_a_proposal_of_another_identity() {
        let read = stored_proposal(Identifier::from([3; 32]));
        let proposal_id = read.id();
        let sdk = sdk_storing(proposal_id, Some(read)).await;
        let wallet = wallet_with_signing_keys_on(sdk, Arc::new(NoPlatformPersistence)).await;

        let error = wallet
            .identity()
            .edit_decision_proposal_with_signer(&owner(), &proposal_id, &retitle(), &available(&[]))
            .await
            .expect_err("another identity's proposal");
        assert!(
            matches!(error, PlatformWalletError::InvalidParameter(ref message)
                if message.contains("belongs to identity")),
            "{error:?}"
        );
    }

    /// The create builds the proposal through rs-sdk, then asks the signer
    /// for a key with the wallet manager lock released: a signer that holds
    /// no key keeps the structured key-unavailable failure, and an owner
    /// the wallet does not hold is refused.
    #[tokio::test]
    async fn should_create_only_for_an_owner_with_a_key_the_signer_holds() {
        let wallet =
            wallet_with_signing_keys_on(proposals_sdk(), Arc::new(NoPlatformPersistence)).await;
        let signer = lock_checking_signer(wallet.identity());

        let error = wallet
            .identity()
            .create_decision_proposal_with_signer(&owner(), &input(), &signer)
            .await
            .expect_err("no key available");
        assert!(
            matches!(error, PlatformWalletError::Sdk(dash_sdk::Error::Protocol(ProtocolError::Generic(ref message)))
                if message.starts_with(SIGNER_KEY_UNAVAILABLE_PREFIX)),
            "{error:?}"
        );

        let stranger = Identifier::from([5; 32]);
        let error = wallet
            .identity()
            .create_decision_proposal_with_signer(&stranger, &input(), &available(&[1]))
            .await
            .expect_err("not an identity of this wallet");
        assert!(
            matches!(error, PlatformWalletError::IdentityNotFound(id) if id == stranger),
            "{error:?}"
        );
    }

    /// Platform's refusal of a stale replace keeps its consensus error, so
    /// the FFI hands the host its code (40106) to read the proposal again.
    #[test]
    fn should_keep_platforms_refusal_of_a_stale_revision() {
        let cause = ConsensusError::StateError(StateError::InvalidDocumentRevisionError(
            InvalidDocumentRevisionError::new(Identifier::from([4; 32]), Some(2), 2),
        ));
        let refusal =
            dash_sdk::Error::StateTransitionBroadcastError(StateTransitionBroadcastError {
                code: 40106,
                message: cause.to_string(),
                cause: Some(cause),
            });
        let error = PlatformWalletError::from(refusal);
        assert!(
            matches!(
                error.consensus_error(),
                Some(ConsensusError::StateError(StateError::InvalidDocumentRevisionError(refused)))
                    if refused.previous_revision() == &Some(2) && refused.desired_revision() == &2
            ),
            "{error:?}"
        );
    }
}
