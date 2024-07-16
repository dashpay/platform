use crate::drive::votes::resolved::votes::resolve::VoteResolver;
use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::identity::masternode_vote::v0::{
    MasternodeVoteTransitionActionV0, PreviousVoteCount,
};
use dpp::state_transition::state_transitions::identity::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl MasternodeVoteTransitionActionV0 {
    pub(crate) fn transform_from_owned_transition(
        value: MasternodeVoteTransitionV0,
        voting_address: [u8; 20],
        masternode_strength: u8,
        previous_resource_vote_choice_to_remove: Option<(ResourceVoteChoice, PreviousVoteCount)>,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let MasternodeVoteTransitionV0 {
            pro_tx_hash,
            voter_identity_id,
            vote,
            nonce,
            ..
        } = value;
        let resolved_vote = vote.resolve_owned(drive, transaction, platform_version)?;
        Ok(MasternodeVoteTransitionActionV0 {
            pro_tx_hash,
            voter_identity_id,
            voting_address,
            vote_strength: masternode_strength,
            vote: resolved_vote,
            previous_resource_vote_choice_to_remove,
            nonce,
        })
    }

    pub(crate) fn transform_from_transition(
        value: &MasternodeVoteTransitionV0,
        voting_address: [u8; 20],
        masternode_strength: u8,
        previous_resource_vote_choice_to_remove: Option<(ResourceVoteChoice, PreviousVoteCount)>,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let MasternodeVoteTransitionV0 {
            pro_tx_hash,
            voter_identity_id,
            vote,
            nonce,
            ..
        } = value;
        let resolved_vote = vote.resolve(drive, transaction, platform_version)?;
        Ok(MasternodeVoteTransitionActionV0 {
            pro_tx_hash: *pro_tx_hash,
            voter_identity_id: *voter_identity_id,
            voting_address,
            vote_strength: masternode_strength,
            vote: resolved_vote,
            previous_resource_vote_choice_to_remove,
            nonce: *nonce,
        })
    }
}
