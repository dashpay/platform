use crate::drive::votes::paths::VotePollPaths;
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::{BatchDeleteApplyType, DirectQueryType};
use dpp::identifier::Identifier;
use dpp::identity::TimestampMillis;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use grovedb::element::reference_path::ReferencePathType::UpstreamRootHeightReference;
use grovedb::{Element, MaybeTree, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    /// Removes the documents and references of ended contests as version 1 does, except that a
    /// contender's document is found by the id its reference names, where version 1 decoded
    /// every contender's document to read its id and failed on one that does not decode.
    /// It reads the contenders among `vote_choices`, the ones the tally returned, and version 1
    /// queried all of them: the same contenders from protocol version 14 on, where a contest
    /// accepts fewer (`max_contenders_per_contest`) than the tally reads
    /// (`maximum_contenders_to_consider`). Selected from protocol version 15.
    // TODO: Use type of struct
    #[allow(clippy::type_complexity)]
    pub(in crate::drive::votes) fn remove_contested_resource_vote_poll_documents_operations_v2(
        &self,
        vote_polls: &[(
            &ContestedDocumentResourceVotePollWithContractInfo,
            &TimestampMillis,
            &BTreeMap<ResourceVoteChoice, Vec<Identifier>>,
        )],
        clean_up_testnet_corrupted_reference_issue: bool,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        for (vote_poll, _, vote_choices) in vote_polls {
            if !clean_up_testnet_corrupted_reference_issue {
                // if we are in clean up we keep all documents
                let documents_storage_path = vote_poll.documents_storage_path_vec();
                for resource_vote_choice in vote_choices.keys() {
                    if !matches!(resource_vote_choice, ResourceVoteChoice::TowardsIdentity(_)) {
                        continue;
                    }
                    let contender_path =
                        vote_poll.contender_path(resource_vote_choice, platform_version)?;
                    // A contender references its document in the contest's document storage
                    // (`make_document_contested_reference`): the reference's last segment is
                    // the document's id
                    let Some(reference) = self.grove_get_raw_optional(
                        contender_path.as_slice().into(),
                        &[0],
                        DirectQueryType::StatefulDirectQuery,
                        transaction,
                        &mut vec![],
                        &platform_version.drive,
                    )?
                    else {
                        // With no reference there is no document id to delete: the clean-up
                        // goes on with the other contenders, as version 1 removed only the
                        // documents its query found
                        continue;
                    };
                    let Element::Reference(UpstreamRootHeightReference(_, reference_path), ..) =
                        reference
                    else {
                        return Err(Error::Drive(DriveError::CorruptedDriveState(
                            "a contender must reference its document".to_string(),
                        )));
                    };
                    let document_id = reference_path.last().ok_or(Error::Drive(
                        DriveError::CorruptedDriveState(
                            "a contender's reference must name its document".to_string(),
                        ),
                    ))?;
                    self.batch_delete(
                        documents_storage_path.as_slice().into(),
                        document_id.as_slice(),
                        BatchDeleteApplyType::StatefulBatchDelete {
                            is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                        },
                        transaction,
                        batch_operations,
                        &platform_version.drive,
                    )?;
                }
            }

            // We also need to delete all the references
            self.remove_contested_resource_vote_poll_contender_references_operations_v1(
                vote_poll,
                vote_choices,
                batch_operations,
                transaction,
                platform_version,
            )?;
        }

        Ok(())
    }
}
