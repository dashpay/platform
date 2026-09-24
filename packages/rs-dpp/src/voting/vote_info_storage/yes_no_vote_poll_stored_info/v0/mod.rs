use crate::block::block_info::BlockInfo;
use crate::voting::vote_info_storage::yes_no_vote_poll_stored_info::{
    YesNoVotePollResult, YesNoVotePollStatus,
};
use crate::voting::vote_polls::yes_no_vote_poll::VotingPower;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use std::fmt;

/// Version 0 of the stored info of a yes/no vote poll: only its status.
#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode, DecodeUntrusted)]
pub struct YesNoVotePollStoredInfoV0 {
    /// Where the poll is in its life.
    pub status: YesNoVotePollStatus,
}

impl fmt::Display for YesNoVotePollStoredInfoV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "YesNoVotePollStoredInfoV0 {{ status: {} }}", self.status)
    }
}

impl YesNoVotePollStoredInfoV0 {
    pub fn new(start_block: BlockInfo) -> YesNoVotePollStoredInfoV0 {
        YesNoVotePollStoredInfoV0 {
            status: YesNoVotePollStatus::Started(start_block),
        }
    }

    pub fn finalize_vote_poll(
        &mut self,
        yes_voting_power: VotingPower,
        no_voting_power: VotingPower,
        abstain_voting_power: VotingPower,
        required_voting_power: VotingPower,
        passed: bool,
        finalization_block: BlockInfo,
    ) -> Result<YesNoVotePollResult, ProtocolError> {
        let YesNoVotePollStatus::Started(start_block) = self.status else {
            return Err(ProtocolError::CorruptedCodeExecution(
                "trying to finalize a yes/no vote poll that is not started".to_string(),
            ));
        };
        let result = YesNoVotePollResult {
            passed,
            yes_voting_power,
            no_voting_power,
            abstain_voting_power,
            required_voting_power,
            start_block,
            finalization_block,
        };
        self.status = YesNoVotePollStatus::Finished(result);
        Ok(result)
    }
}
