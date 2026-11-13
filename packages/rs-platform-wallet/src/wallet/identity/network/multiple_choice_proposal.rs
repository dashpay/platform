//! Multiple choice proposal create and edit, and entering a choice, on
//! `IdentityWallet` (protocol version 15).
//!
//! rs-sdk decides what a multiple choice proposal and a choice are
//! (`dash_sdk::platform::decision_proposals::multiple_choice`): the
//! properties they are created with, refusing an edit that changes
//! nothing, and the revision an edit moves to. The put of a choice states
//! the fund of the contest it joins, read just before it is signed. These
//! operations orchestrate: they resolve the contract, read the proposal,
//! select the signing key, put the document and keep the owner's proven
//! balance, through the steps the yes/no proposal create in
//! `decision_proposal.rs` takes.
//!
//! An edit reads the proposal once and puts that document at the revision
//! after the one it read, exactly as a yes/no proposal edit does: when
//! another edit lands in between, Platform refuses this one
//! (`InvalidDocumentRevisionError`, 40106) instead of letting it overwrite
//! what it never saw. A choice is never edited.

use dash_sdk::platform::decision_proposals::multiple_choice::{
    build_multiple_choice_proposal_edit, ChoiceInput, MultipleChoiceProposalEdit,
    MultipleChoiceProposalInput,
};
use dash_sdk::platform::decision_proposals::DecisionProposalDocument;
use dpp::decision_proposal::multiple_choice::CHOICE_DOCUMENT_TYPE_NAME;
use dpp::decision_proposal::{
    DECISION_PROPOSALS_CONTRACT_ID, MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME,
};
use dpp::document::{Document, DocumentV0Getters};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::Identifier;

use crate::error::PlatformWalletError;

use super::*;

impl IdentityWallet {
    /// Create a multiple choice proposal of `owner_identity_id` asking
    /// what `input` says, and return it as Platform stored it.
    ///
    /// rs-sdk builds the proposal (`Sdk::build_multiple_choice_proposal`).
    /// Platform charges the owner the multiple choice proposal processing
    /// fee (0.2 Dash, `Sdk::multiple_choice_proposal_processing_fee`) on
    /// top of the fees; the create does not state it. The transition is
    /// signed with the owner's first AUTHENTICATION + ECDSA key at a
    /// security level the `multipleChoiceProposal` type accepts that
    /// `signer` can sign with, and the owner's balance the proof carries
    /// is kept. Platform refuses a proposal that gives neither a
    /// description document nor a url. The proposal's id is the id of the
    /// returned document.
    ///
    /// A network below protocol version 15 has no decision proposals
    /// contract, which is an error. SDK and Platform failures, a refusal
    /// included, arrive as [`PlatformWalletError::Sdk`].
    pub async fn create_multiple_choice_proposal_with_signer<S>(
        &self,
        owner_identity_id: &Identifier,
        input: &MultipleChoiceProposalInput,
        signer: &S,
    ) -> Result<Document, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let document_type = self
            .decision_proposals_document_type(MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME)
            .await?;
        let DecisionProposalDocument { document, entropy } = self
            .sdk
            .build_multiple_choice_proposal(*owner_identity_id, input)
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

    /// Give the multiple choice proposal `proposal_id` of
    /// `owner_identity_id` what `edit` changes, and return it as Platform
    /// stored it.
    ///
    /// Reads the proposal once, applies the edit with rs-sdk's
    /// `build_multiple_choice_proposal_edit` (which refuses an edit that
    /// changes nothing and moves to the revision after the one read) and
    /// puts that document as a replace, signed and kept like
    /// [`Self::create_multiple_choice_proposal_with_signer`]. The put
    /// carries the revision after the one read: if another edit lands in
    /// between, Platform refuses this one (`InvalidDocumentRevisionError`,
    /// 40106) rather than overwrite it, and the caller reads the proposal
    /// again. Platform also refuses an edit more than two hours after the
    /// proposal was created, and one that leaves it with neither a
    /// description document nor a url.
    ///
    /// # Errors
    /// * [`PlatformWalletError::DecisionProposalNotFound`] when there is
    ///   no such proposal;
    /// * [`PlatformWalletError::InvalidParameter`] when the proposal
    ///   belongs to another identity, or the edit changes nothing;
    /// * [`PlatformWalletError::Sdk`] for SDK and Platform failures,
    ///   Platform's refusals included.
    pub async fn edit_multiple_choice_proposal_with_signer<S>(
        &self,
        owner_identity_id: &Identifier,
        proposal_id: &Identifier,
        edit: &MultipleChoiceProposalEdit,
        signer: &S,
    ) -> Result<Document, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let document_type = self
            .decision_proposals_document_type(MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME)
            .await?;
        let read = self
            .sdk
            .fetch_multiple_choice_proposal(*proposal_id)
            .await?;
        let replacement =
            multiple_choice_proposal_replacement(owner_identity_id, proposal_id, read, edit)?;
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

    /// Enter the choice `input` describes, owned by `owner_identity_id`,
    /// for the multiple choice proposal it names, and return it as
    /// Platform stored it.
    ///
    /// rs-sdk builds the choice (`Sdk::build_choice`), and its put reads
    /// the fund of the contest among the proposal's choices just before
    /// it is signed and states it (`Sdk::choice_fund_to_join`, doubled
    /// once the vote holds 250 choices and again for every 50 more); the
    /// owner pays it on top of the fees. The first choice opens the vote.
    /// The transition is signed and the owner's proven balance kept like
    /// [`Self::create_multiple_choice_proposal_with_signer`]. Platform
    /// refuses a choice for a proposal that is not open from anyone but
    /// its owner, and one entered once the vote's join window closed. The
    /// choice's id, which a masternode vote towards it names, is the id of
    /// the returned document.
    ///
    /// A network below protocol version 15 has no decision proposals
    /// contract, which is an error. SDK and Platform failures, a refusal
    /// included, arrive as [`PlatformWalletError::Sdk`].
    pub async fn enter_choice_with_signer<S>(
        &self,
        owner_identity_id: &Identifier,
        input: &ChoiceInput,
        signer: &S,
    ) -> Result<Document, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let document_type = self
            .decision_proposals_document_type(CHOICE_DOCUMENT_TYPE_NAME)
            .await?;
        let DecisionProposalDocument { document, entropy } =
            self.sdk.build_choice(*owner_identity_id, input).await?;
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
}

/// The document an edit of the multiple choice proposal `proposal_id`
/// puts: `read`, the proposal as read once for this edit, with what `edit`
/// changes and the revision after the one read. A proposal that is not
/// there, or that belongs to an identity other than `owner_identity_id`,
/// is refused.
fn multiple_choice_proposal_replacement(
    owner_identity_id: &Identifier,
    proposal_id: &Identifier,
    read: Option<Document>,
    edit: &MultipleChoiceProposalEdit,
) -> Result<Document, PlatformWalletError> {
    let proposal = read.ok_or(PlatformWalletError::DecisionProposalNotFound {
        proposal_id: *proposal_id,
    })?;
    if proposal.owner_id() != *owner_identity_id {
        return Err(PlatformWalletError::InvalidParameter(format!(
            "multiple choice proposal {proposal_id} belongs to identity {}, not to \
             {owner_identity_id}",
            proposal.owner_id()
        )));
    }
    build_multiple_choice_proposal_edit(
        &DECISION_PROPOSALS_CONTRACT_ID,
        MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME,
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
    use super::super::decision_proposal::tests::{
        owner, proposals_contract, proposals_sdk, FixedSignatureSigner, STORED_NONCE,
    };
    use super::super::signing_key::tests::{
        available, identity, lock_checking_signer, wallet_with_signing_keys_on,
    };
    use super::*;
    use crate::error::SIGNER_KEY_UNAVAILABLE_PREFIX;
    use crate::wallet::persister::NoPlatformPersistence;
    use dash_sdk::dapi_grpc::platform::v0::{
        GetContestedResourceVoteStateRequest, GetDocumentsRequest,
    };
    use dash_sdk::drive::config::DEFAULT_QUERY_LIMIT;
    use dash_sdk::drive::query::vote_poll_vote_state_query::{
        ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
    };
    use dash_sdk::platform::decision_proposals::multiple_choice::{
        build_multiple_choice_proposal_document, multiple_choice_proposal_vote_poll,
        DescriptionDocument,
    };
    use dash_sdk::platform::transition::broadcast_request::BroadcastRequestForStateTransition;
    use dash_sdk::platform::{DocumentQuery, Query};
    use dash_sdk::query_types::{Contenders, IdentityContractNonceFetcher};
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::decision_proposal::multiple_choice::proposal_properties;
    use dpp::document::{DocumentV0Setters, INITIAL_REVISION};
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::{Bytes32, Value};
    use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
    use dpp::state_transition::batch_transition::BatchTransition;
    use dpp::version::PlatformVersion;
    use dpp::voting::contender_structs::ContenderWithSerializedDocument;
    use dpp::ProtocolError;
    use std::sync::Arc;

    const CREATED_AT: u64 = 1_700_000_000_000;
    const NEW_URL: &str = "https://forum.dash.org/t/block-size-4mb";

    fn description_document() -> DescriptionDocument {
        DescriptionDocument {
            id: Identifier::from([5; 32]),
            contract_id: Identifier::from([6; 32]),
            document_type: "post".to_string(),
        }
    }

    fn input() -> MultipleChoiceProposalInput {
        MultipleChoiceProposalInput {
            title: "Which block size?".to_string(),
            description_document: Some(description_document()),
            url: Some("https://forum.dash.org/t/block-size".to_string()),
            open: true,
        }
    }

    fn choice_input() -> ChoiceInput {
        ChoiceInput {
            proposal_id: Identifier::from([7; 32]),
            title: "Double it".to_string(),
            description: Some("Twice the size it is today.".to_string()),
        }
    }

    /// A multiple choice proposal of `owner_id` built by rs-sdk, as
    /// Platform stores it at its first revision.
    fn stored_proposal(owner_id: Identifier) -> Document {
        let mut document = build_multiple_choice_proposal_document(
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

    /// Points the proposal at its url alone, with a new url.
    fn readdress() -> MultipleChoiceProposalEdit {
        MultipleChoiceProposalEdit {
            description_document: Some(None),
            url: Some(Some(NEW_URL.to_string())),
            ..Default::default()
        }
    }

    /// The query rs-sdk reads the proposal `proposal_id` with.
    fn proposal_query(proposal_id: Identifier) -> DocumentQuery {
        DocumentQuery::new(
            proposals_contract(),
            MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME,
        )
        .expect("the multiple choice proposal type")
        .with_document_id(&proposal_id)
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

    /// The query the put of a choice for `proposal_id` counts the choices
    /// already entered with, to price the fund it states.
    fn contest_fund_query(proposal_id: Identifier) -> ContestedDocumentVotePollDriveQuery {
        ContestedDocumentVotePollDriveQuery {
            vote_poll: multiple_choice_proposal_vote_poll(proposal_id),
            result_type: ContestedDocumentVotePollDriveQueryResultType::VoteTally,
            offset: None,
            limit: Some(DEFAULT_QUERY_LIMIT),
            start_at: None,
            allow_include_locked_and_abstaining_vote_tally: false,
        }
    }

    /// The owner's first key, the one the wallet signs with.
    fn owners_first_key() -> IdentityPublicKey {
        identity(
            Purpose::AUTHENTICATION,
            SecurityLevel::CRITICAL,
            KeyType::ECDSA_SECP256K1,
        )
        .public_keys()
        .get(&1)
        .expect("the owner's first key")
        .clone()
    }

    #[test]
    fn should_move_the_proposal_read_to_its_next_revision_with_only_the_edited_fields() {
        let read = stored_proposal(owner());
        let replacement = multiple_choice_proposal_replacement(
            &owner(),
            &read.id(),
            Some(read.clone()),
            &readdress(),
        )
        .expect("edits");

        assert_eq!(replacement.id(), read.id());
        assert_eq!(replacement.owner_id(), owner());
        assert_eq!(replacement.revision(), Some(INITIAL_REVISION + 1));
        assert_eq!(replacement.created_at(), Some(CREATED_AT));
        let mut expected = read.properties().clone();
        for property in [
            proposal_properties::DESCRIPTION_ID,
            proposal_properties::DESCRIPTION_CONTRACT_ID,
            proposal_properties::DESCRIPTION_DOCUMENT_TYPE,
        ] {
            expected.remove(property);
        }
        expected.insert(
            proposal_properties::URL.to_string(),
            Value::Text(NEW_URL.to_string()),
        );
        assert_eq!(replacement.properties(), &expected);
    }

    #[test]
    fn should_refuse_an_edit_that_changes_nothing() {
        let read = stored_proposal(owner());
        let error = multiple_choice_proposal_replacement(
            &owner(),
            &read.id(),
            Some(read.clone()),
            &MultipleChoiceProposalEdit {
                title: Some(input().title),
                description_document: Some(Some(description_document())),
                url: Some(input().url),
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
            .edit_multiple_choice_proposal_with_signer(
                &owner(),
                &proposal_id,
                &readdress(),
                &FixedSignatureSigner,
            )
            .await
            .expect_err("the mock answers no broadcast");
        assert!(matches!(error, PlatformWalletError::Sdk(_)), "{error:?}");
        assert_eq!(times_read(&mut sdk, proposal_id).await, 1);

        // The proposal as the read returned it: the mock answers with the
        // document re-encoded, so its values come back as Platform returns
        // them rather than as the builder made them.
        let returned = sdk
            .fetch_multiple_choice_proposal(proposal_id)
            .await
            .expect("reads")
            .expect("the stored proposal");
        let contract = proposals_contract();
        let document_type = contract
            .document_type_for_name(MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME)
            .expect("the multiple choice proposal type");
        let mut put_document = returned;
        for property in [
            proposal_properties::DESCRIPTION_ID,
            proposal_properties::DESCRIPTION_CONTRACT_ID,
            proposal_properties::DESCRIPTION_DOCUMENT_TYPE,
        ] {
            put_document.remove(property);
        }
        put_document.set(proposal_properties::URL, Value::Text(NEW_URL.to_string()));
        put_document.set_revision(Some(INITIAL_REVISION + 1));
        let replace = BatchTransition::new_document_replacement_transition_from_document(
            put_document,
            document_type,
            &owners_first_key(),
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
            .edit_multiple_choice_proposal_with_signer(
                &owner(),
                &proposal_id,
                &readdress(),
                &available(&[]),
            )
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
            .edit_multiple_choice_proposal_with_signer(
                &owner(),
                &proposal_id,
                &readdress(),
                &available(&[]),
            )
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
            .create_multiple_choice_proposal_with_signer(&owner(), &input(), &signer)
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
            .create_multiple_choice_proposal_with_signer(&stranger, &input(), &available(&[1]))
            .await
            .expect_err("not an identity of this wallet");
        assert!(
            matches!(error, PlatformWalletError::IdentityNotFound(id) if id == stranger),
            "{error:?}"
        );
    }

    /// Entering a choice selects its key as a proposal create does: a
    /// signer that holds no key keeps the structured key-unavailable
    /// failure, and an owner the wallet does not hold is refused.
    #[tokio::test]
    async fn should_enter_a_choice_only_for_an_owner_with_a_key_the_signer_holds() {
        let wallet =
            wallet_with_signing_keys_on(proposals_sdk(), Arc::new(NoPlatformPersistence)).await;
        let signer = lock_checking_signer(wallet.identity());

        let error = wallet
            .identity()
            .enter_choice_with_signer(&owner(), &choice_input(), &signer)
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
            .enter_choice_with_signer(&stranger, &choice_input(), &available(&[1]))
            .await
            .expect_err("not an identity of this wallet");
        assert!(
            matches!(error, PlatformWalletError::IdentityNotFound(id) if id == stranger),
            "{error:?}"
        );
    }

    /// The put of a choice counts the choices the vote already holds, to
    /// state the fund of the contest, before it reserves the identity
    /// contract nonce and broadcasts. The mock holds no choice yet and
    /// answers no broadcast, so the put fails after sending it.
    #[tokio::test]
    async fn should_read_the_contest_fund_before_putting_a_choice() {
        let proposal_id = choice_input().proposal_id;
        let mut sdk = proposals_sdk();
        sdk.mock()
            .expect_fetch_many::<_, ContenderWithSerializedDocument, _, Contenders>(
                contest_fund_query(proposal_id),
                Some(Contenders::default()),
            )
            .await
            .expect("expects the contest read");
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
            .enter_choice_with_signer(&owner(), &choice_input(), &FixedSignatureSigner)
            .await
            .expect_err("the mock answers no broadcast");
        // The put got as far as the broadcast, which the mock does not answer
        assert!(
            matches!(error, PlatformWalletError::Sdk(_))
                && format!("{error:?}").contains("BroadcastStateTransitionRequest"),
            "{error:?}"
        );

        let contest_read: GetContestedResourceVoteStateRequest =
            Query::<GetContestedResourceVoteStateRequest>::query(
                &contest_fund_query(proposal_id),
                &sdk.query_settings(),
            )
            .expect("encodes");
        assert_eq!(sdk.mock().request_count(&contest_read).await, 1);
    }
}
