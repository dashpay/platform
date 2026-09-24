use crate::consensus::basic::moderation_charter::ModerationCharterRewardSplitNotOneHundredError;
use crate::moderation_charter::SubmittedCharter;
use crate::validation::SimpleConsensusValidationResult;

impl SubmittedCharter {
    #[inline(always)]
    pub(super) fn validate_v0(&self) -> SimpleConsensusValidationResult {
        if self.reward_split.total() != 100 {
            return SimpleConsensusValidationResult::new_with_error(
                ModerationCharterRewardSplitNotOneHundredError::new(
                    self.reward_split.leader,
                    self.reward_split.equal,
                    self.reward_split.actions,
                )
                .into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }
}
