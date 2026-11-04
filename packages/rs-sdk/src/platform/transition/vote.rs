use crate::platform::query::VoteQuery;
use crate::platform::transition::broadcast_request::BroadcastRequestForStateTransition;
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::transition::validation::ensure_valid_state_transition_structure;
use crate::platform::Fetch;
use crate::{Error, Sdk};
use dpp::identifier::MasternodeIdentifiers;
use dpp::identity::hash::IdentityPublicKeyHashMethodsV0;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::Identifier;
use dpp::state_transition::masternode_vote_transition::methods::MasternodeVoteTransitionMethodsV0;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use dpp::voting::votes::resource_vote::accessors::v0::ResourceVoteGettersV0;
use dpp::voting::votes::Vote;
use rs_dapi_client::{DapiRequest, IntoInner};

use super::waitable::Waitable;

#[async_trait::async_trait]
/// A trait for putting a vote on platform
///
/// # `voter_pro_tx_hash` byte order
///
/// Both methods take the masternode's pro_tx_hash in the orientation
/// [`ProTxHash`](dpp::dashcore::ProTxHash) stores — the same bytes Core's RPC
/// hex shows. This is **NOT** interchangeable with [`Txid`](dpp::dashcore::Txid)
/// bytes for the same transaction: `ProTxHash` is declared
/// `#[hash_newtype(forward)]` and `Txid` is not, so the two are exact reverses,
/// and `rpc-json`'s `MasternodeListItem` carries both conventions side by side
/// (`pro_tx_hash: ProTxHash`, `collateral_hash: Txid`).
///
/// The order matters because the voter identity is derived from these bytes
/// (see [`get_voting_identity_id`]) exactly as drive-abci derives it from
/// `masternode.pro_tx_hash.to_byte_array()`. A caller holding wire/`Txid` order
/// — which is what `reg.txid()` yields and what a wallet stores internally —
/// must reverse before calling, or the vote addresses an identity that has
/// never existed and Platform rejects it as having no voter identity.
pub trait PutVote<S: Signer<IdentityPublicKey>>: Waitable {
    /// Puts a vote on platform
    ///
    /// `voter_pro_tx_hash` must be in `ProTxHash` order — see the trait docs.
    async fn put_to_platform(
        &self,
        voter_pro_tx_hash: Identifier,
        voting_public_key: &IdentityPublicKey,
        sdk: &Sdk,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(), Error>;
    /// Puts a vote on platform and waits for the confirmation proof
    ///
    /// `voter_pro_tx_hash` must be in `ProTxHash` order — see the trait docs.
    async fn put_to_platform_and_wait_for_response(
        &self,
        voter_pro_tx_hash: Identifier,
        voting_public_key: &IdentityPublicKey,
        sdk: &Sdk,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Vote, Error>;
}

#[async_trait::async_trait]
impl<S: Signer<IdentityPublicKey>> PutVote<S> for Vote {
    async fn put_to_platform(
        &self,
        voter_pro_tx_hash: Identifier,
        voting_public_key: &IdentityPublicKey,
        sdk: &Sdk,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(), Error> {
        let voting_identity_id = get_voting_identity_id(voter_pro_tx_hash, voting_public_key)?;

        let new_masternode_voting_nonce = sdk
            .get_identity_nonce(voting_identity_id, true, settings)
            .await?;

        let settings = settings.unwrap_or_default();

        let masternode_vote_transition = MasternodeVoteTransition::try_from_vote_with_signer(
            self.clone(),
            signer,
            voter_pro_tx_hash,
            voting_public_key,
            new_masternode_voting_nonce,
            sdk.version(),
            None,
        )
        .await?;
        ensure_valid_state_transition_structure(&masternode_vote_transition, sdk.version())?;
        let request = masternode_vote_transition.broadcast_request_for_state_transition()?;

        request
            .execute(sdk, settings.request_settings)
            .await // TODO: We need better way to handle execution errors
            .into_inner()?;

        Ok(())
    }

    async fn put_to_platform_and_wait_for_response(
        &self,
        voter_pro_tx_hash: Identifier,
        voting_public_key: &IdentityPublicKey,
        sdk: &Sdk,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Vote, Error> {
        let voting_identity_id = get_voting_identity_id(voter_pro_tx_hash, voting_public_key)?;

        let new_masternode_voting_nonce = sdk
            .get_identity_nonce(voting_identity_id, true, settings)
            .await?;

        let settings = settings.unwrap_or_default();

        let Vote::ResourceVote(resource_vote) = self;
        let vote_poll_id = resource_vote.vote_poll().unique_id()?;

        let masternode_vote_transition = MasternodeVoteTransition::try_from_vote_with_signer(
            self.clone(),
            signer,
            voter_pro_tx_hash,
            voting_public_key,
            new_masternode_voting_nonce,
            sdk.version(),
            None,
        )
        .await?;
        ensure_valid_state_transition_structure(&masternode_vote_transition, sdk.version())?;
        let request = masternode_vote_transition.broadcast_request_for_state_transition()?;
        // TODO: Implement retry logic
        let response_result = request
            .execute(sdk, settings.request_settings)
            .await
            .into_inner();

        match response_result {
            Ok(_) => {}
            //todo make this more reliable
            Err(e) => {
                return if e.to_string().contains("already exists") {
                    let vote =
                        Vote::fetch(sdk, VoteQuery::new(voter_pro_tx_hash, vote_poll_id)).await?;
                    let vote = vote.ok_or(Error::Generic(
                        "vote was proved to not exist but was said to exist".to_string(),
                    ))?;
                    // "Already exists" is only the broadcasting node's word, and a node can give
                    // it for a changed vote that is not recorded. The proof shows the vote the
                    // voter holds on this poll, so it confirms the submission only when it is the
                    // submitted vote: the same equality the regular confirmation checks.
                    if &vote == self {
                        Ok(vote)
                    } else {
                        let Vote::ResourceVote(proved) = &vote;
                        Err(Error::InvalidProvedResponse(format!(
                            "vote was said to exist, but the vote proved on its poll is {}, \
                             not the submitted {}: the submitted vote is not recorded",
                            proved.resource_vote_choice(),
                            resource_vote.resource_vote_choice()
                        )))
                    }
                } else {
                    Err(e.into())
                };
            }
        }
        Self::wait_for_response(sdk, masternode_vote_transition, Some(settings)).await
    }
}

/// The voter identity id for `(voter_pro_tx_hash, voting key)`.
///
/// `voter_pro_tx_hash` is used verbatim, so it must already be in `ProTxHash`
/// order (see [`PutVote`]) — this is the same derivation drive-abci performs in
/// `create_voter_identity_v0`, and passing the reversed `Txid` bytes silently
/// yields an identity that does not exist rather than an error.
fn get_voting_identity_id(
    voter_pro_tx_hash: Identifier,
    voting_public_key: &IdentityPublicKey,
) -> Result<Identifier, Error> {
    let pub_key_hash = voting_public_key.public_key_hash()?;

    Ok(Identifier::create_voter_identifier(
        voter_pro_tx_hash.as_bytes(),
        &pub_key_hash,
    ))
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use super::*;
    use crate::platform::transition::masternode_vote_keys::voter_identity_voting_key;
    use dapi_grpc::tonic::Status;
    use dpp::address_funds::AddressWitness;
    use dpp::platform_value::{BinaryData, Value};
    use dpp::state_transition::StateTransition;
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
    use dpp::voting::vote_polls::VotePoll;
    use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
    use dpp::voting::votes::resource_vote::ResourceVote;
    use dpp::ProtocolError;
    use drive_proof_verifier::types::IdentityNonceFetcher;
    use rs_dapi_client::transport::TransportError;
    use rs_dapi_client::{CanRetry, DapiClientError, ExecutionError};

    /// Signs with fixed bytes, so the test can rebuild the exact transition the SDK broadcasts.
    #[derive(Debug)]
    struct FixedSigner;

    #[async_trait::async_trait]
    impl Signer<IdentityPublicKey> for FixedSigner {
        async fn sign(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<BinaryData, ProtocolError> {
            Ok(BinaryData::new(vec![7; 65]))
        }

        async fn sign_create_witness(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            Err(ProtocolError::Generic(
                "a vote has no address witness".to_string(),
            ))
        }

        fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
            true
        }
    }

    fn pro_tx_hash() -> Identifier {
        Identifier::new([1; 32])
    }

    fn voting_key() -> IdentityPublicKey {
        voter_identity_voting_key(&[2; 20])
    }

    fn vote_poll() -> VotePoll {
        VotePoll::ContestedDocumentResourceVotePoll(ContestedDocumentResourceVotePoll {
            contract_id: Identifier::new([3; 32]),
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![
                Value::Text("dash".to_string()),
                Value::Text("alice".to_string()),
            ],
        })
    }

    fn vote(choice: ResourceVoteChoice) -> Vote {
        Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
            vote_poll: vote_poll(),
            resource_vote_choice: choice,
        }))
    }

    fn towards(contender: u8) -> ResourceVoteChoice {
        ResourceVoteChoice::TowardsIdentity(Identifier::new([contender; 32]))
    }

    /// Submits `submitted` to a node that answers the broadcast with "already exists" and proves
    /// `recorded` as the voter's vote on the submitted vote's poll.
    async fn submit_answered_already_exists(
        submitted: &Vote,
        recorded: Option<Vote>,
    ) -> Result<Vote, Error> {
        let mut sdk = Sdk::new_mock();
        let voter_identity_id = get_voting_identity_id(pro_tx_hash(), &voting_key()).unwrap();
        sdk.mock()
            .expect_fetch(voter_identity_id, Some(IdentityNonceFetcher(10)))
            .await
            .unwrap();
        sdk.mock()
            .expect_fetch(
                VoteQuery::new(pro_tx_hash(), submitted.vote_poll_unique_id().unwrap()),
                recorded,
            )
            .await
            .unwrap();

        // The SDK bumps the fetched nonce once before signing.
        let transition: StateTransition = MasternodeVoteTransition::try_from_vote_with_signer(
            submitted.clone(),
            &FixedSigner,
            pro_tx_hash(),
            &voting_key(),
            11,
            sdk.version(),
            None,
        )
        .await
        .unwrap();
        let broadcast = transition.broadcast_request_for_state_transition().unwrap();
        sdk.mock_dapi()
            .lock()
            .await
            .expect(
                &broadcast,
                &Err(ExecutionError {
                    inner: DapiClientError::Transport(TransportError::Grpc(
                        Status::already_exists("state transition already in chain"),
                    )),
                    address: None,
                    retries: 0,
                }),
            )
            .unwrap();

        submitted
            .put_to_platform_and_wait_for_response(
                pro_tx_hash(),
                &voting_key(),
                &sdk,
                &FixedSigner,
                None,
            )
            .await
    }

    /// A node that answers "already exists" to a changed vote must not make the SDK report the
    /// change as cast: the vote proved on the poll is still the previous choice.
    #[tokio::test]
    async fn should_fail_a_vote_change_answered_already_exists_while_the_previous_choice_is_proved()
    {
        let recorded = towards(4);

        let result = submit_answered_already_exists(&vote(towards(5)), Some(vote(recorded))).await;

        let error = result.expect_err("the submitted vote is not the one recorded");
        assert!(
            matches!(&error, Error::InvalidProvedResponse(message)
                if message.contains(&recorded.to_string())),
            "expected a mismatch naming the recorded choice, got {error:?}"
        );
        assert!(
            !error.can_retry(),
            "the proof is sound, so the mismatch is no node failure to retry past"
        );
    }

    /// The duplicate re-broadcast of a vote that was recorded is still confirmed by the proved vote.
    #[tokio::test]
    async fn should_confirm_a_vote_answered_already_exists_when_the_same_vote_is_proved() {
        let submitted = vote(towards(5));

        let confirmed = submit_answered_already_exists(&submitted, Some(submitted.clone()))
            .await
            .expect("the recorded vote is the submitted one");

        assert_eq!(confirmed, submitted);
    }

    /// A vote answered "already exists" whose poll has no proved vote was not recorded.
    #[tokio::test]
    async fn should_fail_a_vote_answered_already_exists_when_no_vote_is_proved() {
        let result = submit_answered_already_exists(&vote(towards(5)), None).await;

        assert!(
            matches!(result, Err(Error::Generic(_))),
            "expected the missing vote to be reported, got {result:?}"
        );
    }
}
