use crate::drive::votes::resolved::votes::resolve::VoteResolver;
use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::identity::masternode_vote::v0::MasternodeVoteTransitionActionV0;
use dpp::state_transition::state_transitions::identity::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl MasternodeVoteTransitionActionV0 {
    pub(crate) fn transform_from_owned_transition(
        value: MasternodeVoteTransitionV0,
        masternode_strength: u8,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let MasternodeVoteTransitionV0 {
            pro_tx_hash,
            vote,
            nonce,
            ..
        } = value;
        let resolved_vote = vote.resolve_owned(drive, transaction, platform_version)?;
        Ok(MasternodeVoteTransitionActionV0 {
            pro_tx_hash,
            vote_strength: masternode_strength,
            vote: resolved_vote,
            nonce,
        })
    }

    pub(crate) fn transform_from_transition(
        value: &MasternodeVoteTransitionV0,
        masternode_strength: u8,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let MasternodeVoteTransitionV0 {
            pro_tx_hash,
            vote,
            nonce,
            ..
        } = value;
        let resolved_vote = vote.resolve(drive, transaction, platform_version)?;
        Ok(MasternodeVoteTransitionActionV0 {
            pro_tx_hash: *pro_tx_hash,
            vote_strength: masternode_strength,
            vote: resolved_vote,
            nonce: *nonce,
        })
    }
}