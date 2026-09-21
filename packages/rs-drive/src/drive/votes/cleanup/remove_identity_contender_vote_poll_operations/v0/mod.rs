use crate::drive::votes::paths::{IDENTITY_CONTENDER_INFO_KEY, VOTING_STORAGE_TREE_KEY};
use crate::drive::votes::resolved::vote_polls::identity_contender_vote_poll::IdentityContenderVotePollPaths;
use crate::drive::votes::ResourceVoteChoiceToKeyTrait;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchDeleteApplyType;
use dpp::identifier::Identifier;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use dpp::ProtocolError;
use grovedb::{MaybeTree, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    pub(super) fn remove_identity_contender_vote_poll_operations_v0(
        &self,
        vote_poll: &IdentityContenderVotePoll,
        votes: &BTreeMap<ResourceVoteChoice, Vec<Identifier>>,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let poll_path = vote_poll.poll_path_vec()?;
        let delete_item = BatchDeleteApplyType::StatefulBatchDelete {
            is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
        };
        for (choice, voters) in votes {
            if matches!(choice, ResourceVoteChoice::Lock) {
                return Err(Error::Protocol(Box::new(ProtocolError::VoteError(
                    "an identity contender vote poll has no lock votes to remove".to_string(),
                ))));
            }
            let choice_path = vote_poll.choice_path_vec(choice)?;
            let votes_path = vote_poll.choice_votes_path_vec(choice)?;
            // The votes, then their sum tree, once empty
            for voter in voters {
                self.batch_delete(
                    votes_path.as_slice().into(),
                    voter.as_slice(),
                    delete_item,
                    transaction,
                    batch_operations,
                    &platform_version.drive,
                )?;
            }
            self.batch_delete(
                choice_path.as_slice().into(),
                &[VOTING_STORAGE_TREE_KEY],
                delete_item,
                transaction,
                batch_operations,
                &platform_version.drive,
            )?;
            // A contender's record, then the choice's tree, once empty
            if matches!(choice, ResourceVoteChoice::TowardsIdentity(_)) {
                self.batch_delete(
                    choice_path.as_slice().into(),
                    &[IDENTITY_CONTENDER_INFO_KEY],
                    delete_item,
                    transaction,
                    batch_operations,
                    &platform_version.drive,
                )?;
            }
            self.batch_delete(
                poll_path.as_slice().into(),
                choice.to_key().as_slice(),
                delete_item,
                transaction,
                batch_operations,
                &platform_version.drive,
            )?;
        }
        Ok(())
    }
}
