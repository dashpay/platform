use crate::drive::votes::paths::YesNoVotePollPaths;
use crate::drive::votes::YesNoAbstainVoteChoiceToKeyTrait;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchDeleteApplyType;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use grovedb::{MaybeTree, TransactionArg};
use std::collections::BTreeMap;

impl Drive {
    pub(super) fn remove_yes_no_vote_poll_votes_operations_v0(
        &self,
        vote_polls: &[(
            &YesNoVotePoll,
            &BTreeMap<YesNoAbstainVoteChoice, Vec<Identifier>>,
        )],
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        for (vote_poll, voters_by_choice) in vote_polls {
            let poll_path = vote_poll.poll_path_vec()?;
            for vote_choice in YesNoAbstainVoteChoice::ALL {
                let votes_path = vote_poll.choice_votes_path_vec(vote_choice)?;
                // Every delete here names its element type, so GroveDB never reads the pending
                // batch for it. Each is built against an empty one: handing `batch_operations`
                // to `batch_delete` would copy every operation already queued on each call and
                // make the clean-up quadratic in the number of voters.
                if let Some(voters) = voters_by_choice.get(&vote_choice) {
                    for voter in voters {
                        let mut delete_operations = vec![];
                        self.batch_delete(
                            votes_path.as_slice().into(),
                            voter.as_slice(),
                            BatchDeleteApplyType::StatefulBatchDelete {
                                is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                            },
                            transaction,
                            &mut delete_operations,
                            &platform_version.drive,
                        )?;
                        batch_operations.append(&mut delete_operations);
                    }
                }
                // The sum tree itself, emptied by the deletes above. As for a contested poll's
                // vote trees, it is not declared a tree: that would make GroveDB read every
                // vote again to prove it empty.
                let mut delete_operations = vec![];
                self.batch_delete(
                    poll_path.as_slice().into(),
                    &[vote_choice.to_tree_key()],
                    BatchDeleteApplyType::StatefulBatchDelete {
                        is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                    },
                    transaction,
                    &mut delete_operations,
                    &platform_version.drive,
                )?;
                batch_operations.append(&mut delete_operations);
            }
        }
        Ok(())
    }
}
