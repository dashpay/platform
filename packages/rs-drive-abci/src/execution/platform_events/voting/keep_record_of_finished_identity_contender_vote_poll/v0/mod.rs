use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::ProTxHash;
use dpp::dashcore_rpc::dashcore_rpc_json::MasternodeType;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::contender_structs::FinalizedResourceVoteChoicesWithVoterInfo;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_info_storage::identity_contender_vote_poll_stored_info::IdentityContenderVotePollStoredInfo;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use drive::grovedb::TransactionArg;
use std::collections::BTreeMap;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn keep_record_of_finished_identity_contender_vote_poll_v0(
        &self,
        block_platform_state: &PlatformState,
        block_info: &BlockInfo,
        vote_poll: &IdentityContenderVotePoll,
        mut stored_info: IdentityContenderVotePollStoredInfo,
        votes: &BTreeMap<ResourceVoteChoice, Vec<Identifier>>,
        winner: Option<Identifier>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let full_masternode_list = block_platform_state.full_masternode_list();
        let resource_vote_choices = votes
            .iter()
            .map(|(resource_vote_choice, voters)| {
                let voters = voters
                    .iter()
                    .map(|pro_tx_hash_identifier| {
                        let strength = match full_masternode_list
                            .get(&ProTxHash::from_byte_array(
                                pro_tx_hash_identifier.to_buffer(),
                            ))
                            .map(|masternode| &masternode.node_type)
                        {
                            Some(MasternodeType::Regular) => 1,
                            Some(MasternodeType::Evo) => 4,
                            None => 0,
                        };
                        (*pro_tx_hash_identifier, strength)
                    })
                    .collect();
                FinalizedResourceVoteChoicesWithVoterInfo {
                    resource_vote_choice: *resource_vote_choice,
                    voters,
                }
            })
            .collect();
        stored_info.resolve(winner, resource_vote_choices, *block_info)?;
        self.drive
            .insert_stored_info_for_identity_contender_vote_poll(
                vote_poll,
                stored_info,
                transaction,
                platform_version,
            )?;
        Ok(())
    }
}
