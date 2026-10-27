use dpp::dashcore_rpc::dashcore_rpc_json::MasternodeType;

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

/// The voting power a masternode's yes/no vote carries: 1 for a masternode, 4 for an evonode,
/// as for contested polls. The strength of each yes/no vote and the masternode list total that a
/// share minimum is measured against both come from here, so they always use the same weights.
/// The contested vote strength of masternode vote `transform_into_action` v0 (protocol versions
/// 1 to 14, and every contested vote at 15) keeps its own copy of these weights.
///
/// Protocol version 15 reads these weights for every yes/no vote (`transform_into_action` v1)
/// and for the masternode list total a share minimum is judged against
/// (`check_for_ended_vote_polls` v1), so once it ships they must not change: other weights for a
/// later version go in a new helper that version selects.
pub(in crate::execution) fn masternode_vote_strength(node_type: &MasternodeType) -> u8 {
    match node_type {
        MasternodeType::Regular => 1,
        MasternodeType::Evo => 4,
    }
}
