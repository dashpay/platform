use super::broadcast::{wait_after_broadcast, BroadcastStateTransition};
use crate::platform::query::VoteQuery;
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
use dpp::voting::votes::Vote;

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
        masternode_vote_transition
            .broadcast(sdk, Some(settings))
            .await?;

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

        let vote_poll_id = self.vote_poll_unique_id(sdk.version())?;

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
        let broadcast = masternode_vote_transition
            .broadcast(sdk, Some(settings))
            .await;
        let duplicate = broadcast
            .as_ref()
            .err()
            .is_some_and(Error::is_already_exists);
        wait_after_broadcast(broadcast, async move {
            if duplicate && matches!(self, Vote::ResourceVote(_)) {
                let vote =
                    Vote::fetch(sdk, VoteQuery::new(voter_pro_tx_hash, vote_poll_id)).await?;
                return vote.ok_or(Error::Generic(
                    "vote was proved to not exist but was said to exist".to_string(),
                ));
            }
            // VoteQuery only reads resource votes; yes/no votes retain their own proof wait.
            Self::wait_for_response(sdk, masternode_vote_transition, Some(settings)).await
        })
        .await
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
mod submission_tests {
    use super::*;
    use crate::platform::transition::masternode_vote_keys::voter_identity_voting_key;
    use crate::sdk::put_test_support::PutFixture;
    use dapi_grpc::tonic::Status;
    use dpp::platform_value::Value;
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
    use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
    use dpp::voting::vote_polls::VotePoll;
    use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
    use dpp::voting::votes::resource_vote::ResourceVote;
    use dpp::voting::votes::yes_no_vote::v0::YesNoVoteV0;
    use dpp::voting::votes::yes_no_vote::YesNoVote;
    use drive_proof_verifier::types::IdentityNonceFetcher;

    #[tokio::test]
    async fn should_wait_for_duplicate_yes_no_vote_without_resource_lookup_or_resubmission() {
        let mut fixture = PutFixture::new().await;
        let key = voter_identity_voting_key(&[3; 20]);
        let pro_tx_hash = Identifier::new([4; 32]);
        let voting_identity_id = get_voting_identity_id(pro_tx_hash, &key).unwrap();
        let vote = Vote::YesNoVote(YesNoVote::V0(YesNoVoteV0 {
            vote_poll_id: Identifier::new([0xc1; 32]),
            vote_choice: YesNoAbstainVoteChoice::Yes,
        }));
        let prepared = MasternodeVoteTransition::try_from_vote_with_signer(
            vote.clone(),
            &fixture.signer,
            pro_tx_hash,
            &key,
            1,
            fixture.sdk.version(),
            None,
        )
        .await
        .unwrap();
        let requests = fixture
            .expect_submission(&prepared, Status::already_exists("vote already exists"))
            .await;
        fixture
            .sdk
            .mock()
            .expect_fetch(voting_identity_id, Some(IdentityNonceFetcher(0)))
            .await
            .unwrap();
        // No resource VoteQuery expectation: that path cannot retrieve yes/no votes.
        let result = vote
            .put_to_platform_and_wait_for_response(
                pro_tx_hash,
                &key,
                &fixture.sdk,
                &fixture.signer,
                PutFixture::settings(),
            )
            .await
            .map(|_| ());
        fixture.assert_calls(&requests, 1, result).await;
    }

    #[tokio::test]
    async fn should_share_broadcast_dispatch_and_nonce_refresh_for_votes() {
        for combined in [false, true] {
            for (status, expected_waits, existing) in [
                (Status::unavailable("acknowledgement lost"), 1, false),
                (
                    Status::resource_exhausted("broadcasting state transition is timed out"),
                    1,
                    false,
                ),
                (Status::resource_exhausted("mempool is full"), 0, false),
                (Status::already_exists("vote already exists"), 0, true),
            ] {
                let mut fixture = PutFixture::new().await;
                let key = voter_identity_voting_key(&[3; 20]);
                let pro_tx_hash = Identifier::new([4; 32]);
                let voting_identity_id = get_voting_identity_id(pro_tx_hash, &key).unwrap();
                let poll = VotePoll::ContestedDocumentResourceVotePoll(
                    ContestedDocumentResourceVotePoll {
                        contract_id: Identifier::new([5; 32]),
                        document_type_name: "domain".into(),
                        index_name: "parentNameAndLabel".into(),
                        index_values: vec![Value::Text("dash".into()), Value::Text("test".into())],
                    },
                );
                let vote = Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
                    vote_poll: poll.clone(),
                    resource_vote_choice: ResourceVoteChoice::Abstain,
                }));
                let prepared = MasternodeVoteTransition::try_from_vote_with_signer(
                    vote.clone(),
                    &fixture.signer,
                    pro_tx_hash,
                    &key,
                    1,
                    fixture.sdk.version(),
                    None,
                )
                .await
                .unwrap();
                let requests = fixture.expect_submission(&prepared, status).await;
                fixture
                    .sdk
                    .mock()
                    .expect_fetch(voting_identity_id, Some(IdentityNonceFetcher(0)))
                    .await
                    .unwrap();
                if existing && combined {
                    let poll_id = poll.unique_id(fixture.sdk.version()).unwrap();
                    fixture
                        .sdk
                        .mock()
                        .expect_fetch(VoteQuery::new(pro_tx_hash, poll_id), None::<Vote>)
                        .await
                        .unwrap();
                }
                let result = if combined {
                    vote.put_to_platform_and_wait_for_response(
                        pro_tx_hash,
                        &key,
                        &fixture.sdk,
                        &fixture.signer,
                        PutFixture::settings(),
                    )
                    .await
                    .map(|_| ())
                } else {
                    vote.put_to_platform(
                        pro_tx_hash,
                        &key,
                        &fixture.sdk,
                        &fixture.signer,
                        PutFixture::settings(),
                    )
                    .await
                };
                if existing && combined {
                    assert!(
                        matches!(&result, Err(Error::StateTransitionResultUnconfirmed(cause)) if matches!(cause.as_ref(), Error::Generic(message) if message.contains("vote was proved to not exist"))),
                        "preserve the legacy fetch fallback: {result:?}"
                    );
                }
                fixture
                    .assert_calls(&requests, if combined { expected_waits } else { 0 }, result)
                    .await;
                // Shared broadcast invalidates the reservation after final send failure.
                // Without the fetch expectation the next nonce read must fail, not use stale cache.
                fixture
                    .sdk
                    .mock()
                    .remove_fetch_expectation::<IdentityNonceFetcher, _>(voting_identity_id)
                    .await;
                assert!(fixture
                    .sdk
                    .get_identity_nonce(voting_identity_id, false, PutFixture::settings())
                    .await
                    .is_err());
            }
        }
    }
}
