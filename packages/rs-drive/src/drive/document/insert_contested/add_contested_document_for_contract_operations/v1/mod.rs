use crate::drive::document::ContestWindows;
use crate::drive::votes::paths::{
    vote_contested_resource_end_date_queries_at_time_tree_path_vec,
    vote_end_date_queries_tree_path_vec,
};
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQueryResultType, ResolvedContestedDocumentVotePollDriveQuery,
};
use crate::util::grove_operations::BatchDeleteUpTreeApplyType;
use crate::util::object_size_info::DocumentAndContractInfo;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::Network;
use dpp::data_contract::document_type::ContestedIndexResolution;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::{
    ContestedDocumentVotePollStatus, ContestedDocumentVotePollStoredInfo,
    ContestedDocumentVotePollStoredInfoV0Getters,
};
use dpp::voting::vote_polls::VotePoll;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, MaybeTree, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Gathers the operations to add a contested document to a contract.
    ///
    /// Version 1 (protocol version 14) reads the contested index's resolution. A contest
    /// resolved without locking (`MasternodeVoteNoLocking`) ends when its join window closes
    /// while it has a single contender; the first additional contender moves its end date to
    /// the full poll duration, opening the vote window.
    ///
    /// A moderation election (an `electedCharter` contest) runs on the join window and the
    /// vote window its target contract declares, on every network; every other contest runs on
    /// the generic windows of the version tables.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_contested_document_for_contract_operations_v1(
        &self,
        document_and_contract_info: DocumentAndContractInfo,
        contested_document_resource_vote_poll: ContestedDocumentResourceVotePollWithContractInfo,
        insert_without_check: bool,
        block_info: &BlockInfo,
        also_insert_vote_poll_stored_info: Option<ContestedDocumentVotePollStoredInfo>,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_contested_document_tree_levels_up_to_contract(
                document_and_contract_info.contract,
                Some(document_and_contract_info.document_type),
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        self.add_contested_document_to_primary_storage(
            &document_and_contract_info,
            insert_without_check,
            estimated_costs_only_with_layer_info,
            transaction,
            &mut batch_operations,
            platform_version,
        )?;

        let generic_windows = match self.config.network {
            Network::Mainnet => ContestWindows {
                join_window_ms: platform_version
                    .dpp
                    .validation
                    .voting
                    .allow_other_contenders_time_mainnet_ms,
                poll_duration_ms: platform_version
                    .dpp
                    .voting_versions
                    .default_vote_poll_time_duration_mainnet_ms,
            },
            _ => ContestWindows {
                join_window_ms: platform_version
                    .dpp
                    .validation
                    .voting
                    .allow_other_contenders_time_testing_ms,
                poll_duration_ms: platform_version
                    .dpp
                    .voting_versions
                    .default_vote_poll_time_duration_test_network_ms,
            },
        };
        let estimating = estimated_costs_only_with_layer_info.is_some();

        let no_locking = contested_document_resource_vote_poll
            .index()?
            .contested_index
            .as_ref()
            .map(|contested| contested.resolution)
            == Some(ContestedIndexResolution::MasternodeVoteNoLocking);

        let contest_already_existed = self.add_contested_indices_for_contract_operations(
            &document_and_contract_info,
            previous_batch_operations,
            estimated_costs_only_with_layer_info,
            transaction,
            &mut batch_operations,
            platform_version,
        )?;

        let owner_id = document_and_contract_info.owned_document_info.owner_id;

        if !contest_already_existed {
            if let Some(vote_poll_stored_start_info) = also_insert_vote_poll_stored_info {
                let mut operations = self
                    .insert_stored_info_for_contested_resource_vote_poll_operations(
                        &contested_document_resource_vote_poll,
                        vote_poll_stored_start_info,
                        platform_version,
                    )?;
                batch_operations.append(&mut operations);
            }

            let windows = self.contest_windows_v1(
                &contested_document_resource_vote_poll,
                generic_windows,
                estimating,
                block_info,
                transaction,
                &mut batch_operations,
                platform_version,
            )?;

            // Without locking, a contest runs only to the end of its join window until a
            // second contender joins; with locking, it always runs the full poll duration
            // so the masternodes may lock a single contender out
            let end_date = if no_locking {
                block_info.time_ms.saturating_add(windows.join_window_ms)
            } else {
                block_info.time_ms.saturating_add(windows.poll_duration_ms)
            };

            self.add_vote_poll_end_date_query_operations(
                owner_id,
                VotePoll::ContestedDocumentResourceVotePoll(
                    contested_document_resource_vote_poll.into(),
                ),
                end_date,
                block_info,
                estimated_costs_only_with_layer_info,
                previous_batch_operations,
                &mut batch_operations,
                transaction,
                platform_version,
            )?;
        } else if no_locking && !estimating {
            // The first additional contender opens the vote window: the end date moves from
            // the end of the join window to the full poll duration. Later contenders find
            // it there already. An estimation never reaches this branch, since a stateless
            // insert reports every contest as new.
            let existing_contenders = ResolvedContestedDocumentVotePollDriveQuery {
                vote_poll: (&contested_document_resource_vote_poll).into(),
                result_type: ContestedDocumentVotePollDriveQueryResultType::VoteTally,
                offset: None,
                limit: Some(2),
                start_at: None,
                allow_include_locked_and_abstaining_vote_tally: false,
            }
            .execute(self, transaction, &mut batch_operations, platform_version)?
            .contenders
            .len();

            if existing_contenders == 1 {
                let (fee_result, stored_info) = self
                    .fetch_contested_document_vote_poll_stored_info(
                        &contested_document_resource_vote_poll,
                        Some(&block_info.epoch),
                        transaction,
                        platform_version,
                    )?;
                if let Some(fee_result) = fee_result {
                    batch_operations
                        .push(LowLevelDriveOperation::PreCalculatedFeeResult(fee_result));
                }
                let Some(stored_info) = stored_info else {
                    return Err(Error::Drive(DriveError::CorruptedDriveState(
                        "a contest with a contender has no stored info".to_string(),
                    )));
                };
                let ContestedDocumentVotePollStatus::Started(start_block) =
                    stored_info.vote_poll_status()
                else {
                    return Err(Error::Drive(DriveError::CorruptedDriveState(
                        "a contest accepting a contender has not started".to_string(),
                    )));
                };

                // The same windows the contest started on: a moderation election's target
                // declared them at its creation and can never change them
                let windows = self.contest_windows_v1(
                    &contested_document_resource_vote_poll,
                    generic_windows,
                    estimating,
                    block_info,
                    transaction,
                    &mut batch_operations,
                    platform_version,
                )?;
                let join_end = start_block.time_ms.saturating_add(windows.join_window_ms);
                let vote_end = start_block.time_ms.saturating_add(windows.poll_duration_ms);
                let vote_poll = VotePoll::ContestedDocumentResourceVotePoll(
                    contested_document_resource_vote_poll.into(),
                );

                if join_end != vote_end {
                    let unique_id = vote_poll.unique_id()?;
                    // The join-window entry goes, and its time tree with it when it was
                    // the only entry at that time
                    self.batch_delete_up_tree_while_empty(
                        KeyInfoPath::from_known_owned_path(
                            vote_contested_resource_end_date_queries_at_time_tree_path_vec(
                                join_end,
                            ),
                        ),
                        unique_id.as_slice(),
                        Some(vote_end_date_queries_tree_path_vec().len() as u16),
                        BatchDeleteUpTreeApplyType::StatefulBatchDelete {
                            is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                        },
                        transaction,
                        &*previous_batch_operations,
                        &mut batch_operations,
                        &platform_version.drive,
                    )?;

                    self.add_vote_poll_end_date_query_operations(
                        owner_id,
                        vote_poll,
                        vote_end,
                        block_info,
                        estimated_costs_only_with_layer_info,
                        previous_batch_operations,
                        &mut batch_operations,
                        transaction,
                        platform_version,
                    )?;
                }
            }
        }

        Ok(batch_operations)
    }

    /// The windows of a contest: those its target contract declares for a moderation election,
    /// `generic_windows` for every other contest. The read of the target is billed into
    /// `batch_operations`. An estimation reads nothing: the end date it writes has the same
    /// size whatever the windows.
    #[allow(clippy::too_many_arguments)]
    fn contest_windows_v1(
        &self,
        contested_document_resource_vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        generic_windows: ContestWindows,
        estimating: bool,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<ContestWindows, Error> {
        if estimating {
            return Ok(generic_windows);
        }
        let (fee_result, charter_election_windows) = self.fetch_charter_election_windows(
            contested_document_resource_vote_poll,
            &block_info.epoch,
            transaction,
            platform_version,
        )?;
        if let Some(fee_result) = fee_result {
            batch_operations.push(LowLevelDriveOperation::PreCalculatedFeeResult(fee_result));
        }
        Ok(charter_election_windows.unwrap_or(generic_windows))
    }
}
