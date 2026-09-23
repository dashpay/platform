use crate::consensus::basic::moderation_charter::{
    ModerationCharterDescriptionTooLongError, ModerationCharterRewardSplitNotOneHundredError,
};
use crate::moderation_charter::SubmittedCharter;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl SubmittedCharter {
    #[inline(always)]
    pub(super) fn validate_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
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

        let max_description_length = platform_version
            .system_limits
            .max_moderation_charter_description_length;
        if self.description.len() > max_description_length as usize {
            return SimpleConsensusValidationResult::new_with_error(
                ModerationCharterDescriptionTooLongError::new(
                    self.description.len() as u64,
                    max_description_length,
                )
                .into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }
}
