use crate::drive::votes::paths::YesNoVotePollPaths;
use crate::drive::votes::YesNoAbstainVoteChoiceToKeyTrait;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::{GroveOp, KeyInfoPath, QualifiedGroveDbOp, SubelementsDeletionBehavior};
use grovedb::TreeType;

impl Drive {
    pub(super) fn remove_yes_no_vote_poll_votes_operations_v0(
        &self,
        vote_polls: &[&YesNoVotePoll],
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        _platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        for vote_poll in vote_polls {
            let poll_path = KeyInfoPath::from_known_owned_path(vote_poll.poll_path_vec()?);
            for vote_choice in YesNoAbstainVoteChoice::ALL {
                // Each choice's sum tree holds only the votes, one sum item per voter, and a
                // finished poll is never opened again, so the tree is dropped whole without
                // reading or deleting its votes one by one (GroveDB's flat drop, which the
                // grove version of protocol version 15 accepts in a batch).
                batch_operations.push(LowLevelDriveOperation::GroveOperation(QualifiedGroveDbOp {
                    path: poll_path.clone(),
                    key: Some(KeyInfo::KnownKey(vec![vote_choice.to_tree_key()])),
                    op: GroveOp::DeleteTreeDontCheckForBackwardsReferences(
                        TreeType::SumTree,
                        SubelementsDeletionBehavior::DropFlat,
                    ),
                }));
            }
        }
        Ok(())
    }
}
