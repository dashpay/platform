use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::ProtocolError;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use drive::fees::op::LowLevelDriveOperation;
use drive::grovedb::TransactionArg;
use std::collections::BTreeMap;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks for ended vote polls
    #[inline(always)]
    // TODO: Use type or struct
    #[allow(clippy::type_complexity)]
    pub(super) fn clean_up_after_contested_resources_vote_polls_end_v0(
        &self,
        vote_polls: Vec<(
            &ContestedDocumentResourceVotePollWithContractInfo,
            &TimestampMillis,
            &BTreeMap<ResourceVoteChoice, Vec<Identifier>>,
        )>,
        clean_up_testnet_corrupted_reference_issue: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let operations = self.clean_up_after_contested_resources_vote_polls_end_operations_v0(
            vote_polls.as_slice(),
            clean_up_testnet_corrupted_reference_issue,
            transaction,
            platform_version,
        )?;
        if !operations.is_empty() {
            self.drive.apply_batch_low_level_drive_operations(
                None,
                transaction,
                operations,
                &mut vec![],
                &platform_version.drive,
            )?;
        }

        Ok(())
    }
    /// Checks for ended vote polls
    #[inline(always)]
    // TODO: Use type or struct
    #[allow(clippy::type_complexity)]
    pub(super) fn clean_up_after_contested_resources_vote_polls_end_operations_v0(
        &self,
        vote_polls: &[(
            &ContestedDocumentResourceVotePollWithContractInfo,
            &TimestampMillis,
            &BTreeMap<ResourceVoteChoice, Vec<Identifier>>,
        )],
        clean_up_testnet_corrupted_reference_issue: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut operations = vec![];

        // We remove the end date query
        self.drive
            .remove_contested_resource_vote_poll_end_date_query_operations(
                vote_polls,
                &mut operations,
                transaction,
                platform_version,
            )?;

        // We remove the votes from under the contenders votes received
        self.drive
            .remove_contested_resource_vote_poll_votes_operations(
                vote_polls,
                true,
                &mut operations,
                transaction,
                platform_version,
            )?;

        // We remove the documents that contenders have
        self.drive
            .remove_contested_resource_vote_poll_documents_operations(
                vote_polls,
                clean_up_testnet_corrupted_reference_issue,
                &mut operations,
                transaction,
                platform_version,
            )?;

        // We remove the contenders
        self.drive
            .remove_contested_resource_vote_poll_contenders_operations(
                vote_polls,
                &mut operations,
                transaction,
                platform_version,
            )?;

        let vote_poll_ids = vote_polls
            .iter()
            .map(|(vote_poll, _, _)| Ok((*vote_poll, vote_poll.unique_id()?)))
            .collect::<Result<
                Vec<(
                    &ContestedDocumentResourceVotePollWithContractInfo,
                    Identifier,
                )>,
                ProtocolError,
            >>()?;

        let mut identity_to_vote_ids_map: BTreeMap<&Identifier, Vec<&Identifier>> = BTreeMap::new();

        for (vote_poll, _, voters_for_contender) in vote_polls {
            let vote_id = vote_poll_ids
                .iter()
                .find_map(|(vp, vid)| if vp == vote_poll { Some(vid) } else { None })
                .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "the vote poll must exist in this lookup table",
                )))?;

            for identifiers in voters_for_contender.values() {
                for identity_id in identifiers {
                    identity_to_vote_ids_map
                        .entry(identity_id)
                        .or_default()
                        .push(vote_id);
                }
            }
        }

        for (identity, vote_ids) in identity_to_vote_ids_map {
            // We remove the identity votes given
            self.drive
                .remove_specific_vote_references_given_by_identity(
                    identity,
                    vote_ids.as_slice(),
                    &mut operations,
                    transaction,
                    platform_version,
                )?;
        }

        if clean_up_testnet_corrupted_reference_issue {
            self.drive.remove_contested_resource_info_operations(
                vote_polls,
                &mut operations,
                transaction,
                platform_version,
            )?;
            // We remove the last index
            self.drive
                .remove_contested_resource_top_level_index_operations(
                    vote_polls,
                    &mut operations,
                    transaction,
                    platform_version,
                )?;
        }

        Ok(operations)
    }
}
