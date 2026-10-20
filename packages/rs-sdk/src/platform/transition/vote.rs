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
