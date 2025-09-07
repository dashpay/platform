use crate::platform::query::VoteQuery;
use crate::platform::transition::broadcast_request::BroadcastRequestForStateTransition;
use crate::platform::transition::put_settings::PutSettings;
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
pub trait PutVote<S: Signer>: Waitable {
    /// Puts an identity on platform
    async fn put_to_platform(
        &self,
        voter_pro_tx_hash: Identifier,
        voting_public_key: &IdentityPublicKey,
        sdk: &Sdk,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(), Error>;
    /// Puts an identity on platform and waits for the confirmation proof
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
impl<S: Signer> PutVote<S> for Vote {
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
        )?;
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
        )?;
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
                    vote.ok_or(Error::Generic(
                        "vote was proved to not exist but was said to exist".to_string(),
                    ))
                } else {
                    Err(e.into())
                }
            }
        }
        Self::wait_for_response(sdk, masternode_vote_transition, Some(settings)).await
    }
}

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
