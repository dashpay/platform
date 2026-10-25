use crate::drive::votes::paths::vote_decisions_active_polls_poll_tree_path_vec;
use crate::drive::votes::YesNoAbstainVoteChoiceToKeyTrait;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use grovedb::batch::{QualifiedGroveDbOp, SubelementsDeletionBehavior};
use grovedb::TreeType;

impl Drive {
    pub(super) fn remove_yes_no_vote_poll_votes_operations_v0(
        &self,
        vote_poll_ids: &[Identifier],
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        _platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        for vote_poll_id in vote_poll_ids {
            let poll_path = vote_decisions_active_polls_poll_tree_path_vec(vote_poll_id.as_bytes());
            for vote_choice in YesNoAbstainVoteChoice::ALL {
                // Each choice's sum tree holds only the votes, one sum item per voter, and a
                // finished poll is never opened again, so the tree is dropped whole without
                // reading or deleting its votes one by one (GroveDB's flat drop, which the
                // grove version of protocol version 15 accepts in a batch).
                batch_operations.push(LowLevelDriveOperation::GroveOperation(
                    QualifiedGroveDbOp::delete_tree_op(
                        poll_path.clone(),
                        vec![vote_choice.to_tree_key()],
                        TreeType::SumTree,
                        SubelementsDeletionBehavior::DropFlat,
                    )
                    .dont_check_for_backwards_references(),
                ));
            }
        }
        Ok(())
    }
}
