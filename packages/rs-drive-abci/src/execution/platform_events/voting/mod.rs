use dpp::dashcore_rpc::dashcore_rpc_json::MasternodeType;
use dpp::version::PlatformVersion;

mod award_document_to_winner;
mod check_for_ended_vote_polls;
mod clean_up_after_contested_resources_vote_polls_end;
mod clean_up_after_vote_polls_end;
mod clean_up_after_yes_no_vote_polls_end;
mod keep_record_of_finished_yes_no_vote_poll;
mod keep_record_of_vote_poll;
mod on_yes_no_vote_poll_finished;
mod remove_votes_for_removed_masternodes;
mod run_dao_platform_events;
mod tally_votes_for_contested_document_resource_vote_poll;
mod tally_votes_for_yes_no_vote_poll;

/// The voting power a masternode's yes/no vote carries, read from the protocol version's
/// `SystemLimits` (`yes_no_vote_poll_masternode_vote_strength` and
/// `yes_no_vote_poll_evonode_vote_strength`: 1 for a masternode, 4 for an evonode, as for
/// contested polls). The strength of each yes/no vote (`transform_into_action` v1) and the
/// masternode list total a share minimum is measured against (`check_for_ended_vote_polls` v1)
/// both come from here, so they always use the same weights; a later protocol version changes
/// them in its own `SystemLimits` table. The contested vote strength of masternode vote
/// `transform_into_action` v0 (protocol versions 1 to 14, and every contested vote at 15) keeps
/// its own copy of these weights.
pub(in crate::execution) fn masternode_vote_strength(
    node_type: &MasternodeType,
    platform_version: &PlatformVersion,
) -> u8 {
    let limits = &platform_version.system_limits;
    match node_type {
        MasternodeType::Regular => limits.yes_no_vote_poll_masternode_vote_strength,
        MasternodeType::Evo => limits.yes_no_vote_poll_evonode_vote_strength,
    }
}
