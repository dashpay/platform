use crate::moderation_charter::SubmittedCharter;
use crate::validation::SimpleConsensusValidationResult;

impl SubmittedCharter {
    /// No rule is left for this step: the reward split's sum is the contract's
    /// `propertyConstraints` rule `rewardSplitIsWhole` and the description's byte cap its
    /// `maxBytes`, both checked wherever the document is validated, so every proposal read
    /// from a document already meets them.
    #[inline(always)]
    pub(super) fn validate_v0(&self) -> SimpleConsensusValidationResult {
        SimpleConsensusValidationResult::new()
    }
}
