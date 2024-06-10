use crate::platform::transition::broadcast_request::BroadcastRequestForStateTransition;
use crate::{Error, Sdk};

use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::Identifier;
use dpp::state_transition::masternode_vote_transition::methods::MasternodeVoteTransitionMethodsV0;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;

use crate::platform::block_info_from_metadata::block_info_from_metadata;
use crate::platform::transition::put_settings::PutSettings;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::voting::votes::Vote;
use drive::drive::Drive;
use rs_dapi_client::DapiRequest;

#[async_trait::async_trait]
/// A trait for putting a vote on platform
pub trait PutVote<S: Signer> {
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
        let new_masternode_voting_nonce = sdk
            .get_identity_nonce(voter_pro_tx_hash, true, settings)
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

        request.execute(sdk, settings.request_settings).await?;

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
        let new_masternode_voting_nonce = sdk
            .get_identity_nonce(voter_pro_tx_hash, true, settings)
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

        let response_result = request.execute(sdk, settings.request_settings).await;

        match response_result {
            Ok(_) => {}
            //todo make this more reliable
            Err(e) => {
                if e.to_string().contains("already exists") {
                    todo!("fetch the vote and return it");
                    // let vote = Vote::fetch(sdk, voter_pro_tx_hash).await?;
                    // return vote.ok_or(Error::DapiClientError(
                    //     "vote was proved to not exist but was said to exist".to_string(),
                    // ));
                } else {
                    return Err(e.into());
                }
            }
        }

        let request = masternode_vote_transition.wait_for_state_transition_result_request()?;
        let response = request.execute(sdk, settings.request_settings).await?;

        let block_info = block_info_from_metadata(response.metadata()?)?;
        let proof = response.proof_owned()?;

        let (_, result) = Drive::verify_state_transition_was_executed_with_proof(
            &masternode_vote_transition,
            &block_info,
            proof.grovedb_proof.as_slice(),
            &|_| Ok(None),
            sdk.version(),
        )?;

        //todo verify

        match result {
            StateTransitionProofResult::VerifiedMasternodeVote(vote) => Ok(vote),
            _ => Err(Error::DapiClientError(
                "proved something that was not a vote".to_string(),
            )),
        }
    }
}
