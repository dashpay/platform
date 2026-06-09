use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::block_info::BlockInfo;
use dpp::state_transition::StateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

/// A trait for validating identity nonce rules within a state transition.
pub(crate) trait StateTransitionIdentityNonceValidationV0 {
    /// Validates the identity nonce constraints for this state transition.
    ///
    /// # Arguments
    ///
    /// * `platform` – Reference to the platform state.
    /// * `block_info` – Information about the current block.
    /// * `tx` – The raw transaction argument.
    /// * `execution_context` – Execution context for the state transition.
    /// * `platform_version` – The active platform version.
    ///
    /// # Returns
    ///
    /// Returns a [`SimpleConsensusValidationResult`] on success
    /// or an [`Error`] if validation fails.
    fn validate_identity_nonces(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

pub(crate) trait StateTransitionHasIdentityNonceValidationV0 {
    /// True if the state transition has identity nonces validation.
    fn has_identity_nonce_validation(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error>;
}

impl StateTransitionIdentityNonceValidationV0 for StateTransition {
    fn validate_identity_nonces(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match self {
            StateTransition::Batch(st) => st.validate_identity_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::DataContractCreate(st) => st.validate_identity_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::DataContractUpdate(st) => st.validate_identity_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::IdentityUpdate(st) => st.validate_identity_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::IdentityCreditTransfer(st) => st.validate_identity_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::IdentityCreditWithdrawal(st) => st.validate_identity_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::MasternodeVote(st) => st.validate_identity_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::IdentityCreditTransferToAddresses(st) => st.validate_identity_nonces(
                platform,
                block_info,
                tx,
                execution_context,
                platform_version,
            ),
            StateTransition::AddressCreditWithdrawal(_)
            | StateTransition::AddressFundingFromAssetLock(_)
            | StateTransition::IdentityCreateFromAddresses(_)
            | StateTransition::IdentityTopUpFromAddresses(_)
            | StateTransition::AddressFundsTransfer(_)
            | StateTransition::IdentityCreate(_)
            | StateTransition::IdentityTopUp(_)
            | StateTransition::Shield(_)
            | StateTransition::ShieldedTransfer(_)
            | StateTransition::Unshield(_)
            | StateTransition::ShieldFromAssetLock(_)
            | StateTransition::ShieldedWithdrawal(_)
            | StateTransition::IdentityCreateFromShieldedPool(_) => {
                Ok(SimpleConsensusValidationResult::new())
            }
        }
    }
}

impl StateTransitionHasIdentityNonceValidationV0 for StateTransition {
    fn has_identity_nonce_validation(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .has_nonce_validation
        {
            0 => {
                let has_nonce_validation = matches!(
                    self,
                    StateTransition::Batch(_)
                        | StateTransition::DataContractCreate(_)
                        | StateTransition::DataContractUpdate(_)
                        | StateTransition::IdentityUpdate(_)
                        | StateTransition::IdentityCreditTransfer(_)
                        | StateTransition::IdentityCreditWithdrawal(_)
                );

                Ok(has_nonce_validation)
            }
            1 => {
                // Preferably to use match without wildcard arm (_) to avoid missing cases
                // in the future when new state transitions are added
                let has_nonce_validation = match self {
                    StateTransition::Batch(_)
                    | StateTransition::DataContractCreate(_)
                    | StateTransition::DataContractUpdate(_)
                    | StateTransition::IdentityUpdate(_)
                    | StateTransition::IdentityCreditTransfer(_)
                    | StateTransition::IdentityCreditWithdrawal(_)
                    | StateTransition::MasternodeVote(_)
                    | StateTransition::IdentityCreditTransferToAddresses(_) => true,
                    StateTransition::IdentityCreate(_)
                    | StateTransition::IdentityTopUp(_)
                    | StateTransition::IdentityCreateFromAddresses(_)
                    | StateTransition::IdentityTopUpFromAddresses(_)
                    | StateTransition::AddressFundsTransfer(_)
                    | StateTransition::AddressFundingFromAssetLock(_)
                    | StateTransition::AddressCreditWithdrawal(_)
                    | StateTransition::Shield(_)
                    | StateTransition::ShieldedTransfer(_)
                    | StateTransition::Unshield(_)
                    | StateTransition::ShieldFromAssetLock(_)
                    | StateTransition::ShieldedWithdrawal(_)
                    | StateTransition::IdentityCreateFromShieldedPool(_) => false,
                };

                Ok(has_nonce_validation)
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "StateTransition::has_nonce_validation".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

// Version dispatch tests for has_identity_nonce_validation were intentionally removed.
// The version-specific routing (v0 vs v1) is covered by strategy tests that exercise
// the processor at the platform version used by the test harness.

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::state_transition::batch_transition::BatchTransition;
    use dpp::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
    use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
    use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
    use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
    use dpp::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
    use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
    use dpp::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
    use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
    use dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
    use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
    use dpp::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
    use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
    use dpp::version::PlatformVersion;

    /// Helper to build a Batch StateTransition from a default V0.
    fn batch_st() -> StateTransition {
        StateTransition::Batch(BatchTransition::V0(Default::default()))
    }

    fn identity_update_st() -> StateTransition {
        StateTransition::IdentityUpdate(IdentityUpdateTransition::from(
            IdentityUpdateTransitionV0::default(),
        ))
    }

    fn identity_credit_transfer_st() -> StateTransition {
        StateTransition::IdentityCreditTransfer(IdentityCreditTransferTransition::from(
            IdentityCreditTransferTransitionV0::default(),
        ))
    }

    fn identity_credit_withdrawal_st() -> StateTransition {
        StateTransition::IdentityCreditWithdrawal(IdentityCreditWithdrawalTransition::from(
            IdentityCreditWithdrawalTransitionV0::default(),
        ))
    }

    fn identity_create_st() -> StateTransition {
        StateTransition::IdentityCreate(IdentityCreateTransition::from(
            IdentityCreateTransitionV0::default(),
        ))
    }

    fn identity_top_up_st() -> StateTransition {
        StateTransition::IdentityTopUp(IdentityTopUpTransition::from(
            IdentityTopUpTransitionV0::default(),
        ))
    }

    fn masternode_vote_st() -> StateTransition {
        StateTransition::MasternodeVote(MasternodeVoteTransition::from(
            MasternodeVoteTransitionV0::default(),
        ))
    }

    // ---- has_identity_nonce_validation with version 0 (PlatformVersion::first) ----

    #[test]
    fn has_nonce_validation_v0_batch_returns_true() {
        let platform_version = PlatformVersion::first();
        if platform_version
            .drive_abci
            .validation_and_processing
            .has_nonce_validation
            != 0
        {
            return;
        }
        let result = batch_st()
            .has_identity_nonce_validation(platform_version)
            .expect("should not error");
        assert!(result);
    }

    #[test]
    fn has_nonce_validation_v0_identity_update_returns_true() {
        let platform_version = PlatformVersion::first();
        if platform_version
            .drive_abci
            .validation_and_processing
            .has_nonce_validation
            != 0
        {
            return;
        }
        let result = identity_update_st()
            .has_identity_nonce_validation(platform_version)
            .expect("should not error");
        assert!(result);
    }

    #[test]
    fn has_nonce_validation_v0_identity_credit_transfer_returns_true() {
        let platform_version = PlatformVersion::first();
        if platform_version
            .drive_abci
            .validation_and_processing
            .has_nonce_validation
            != 0
        {
            return;
        }
        let result = identity_credit_transfer_st()
            .has_identity_nonce_validation(platform_version)
            .expect("should not error");
        assert!(result);
    }

    #[test]
    fn has_nonce_validation_v0_identity_credit_withdrawal_returns_true() {
        let platform_version = PlatformVersion::first();
        if platform_version
            .drive_abci
            .validation_and_processing
            .has_nonce_validation
            != 0
        {
            return;
        }
        let result = identity_credit_withdrawal_st()
            .has_identity_nonce_validation(platform_version)
            .expect("should not error");
        assert!(result);
    }

    #[test]
    fn has_nonce_validation_v0_identity_create_returns_false() {
        let platform_version = PlatformVersion::first();
        if platform_version
            .drive_abci
            .validation_and_processing
            .has_nonce_validation
            != 0
        {
            return;
        }
        let result = identity_create_st()
            .has_identity_nonce_validation(platform_version)
            .expect("should not error");
        assert!(!result);
    }

    #[test]
    fn has_nonce_validation_v0_identity_top_up_returns_false() {
        let platform_version = PlatformVersion::first();
        if platform_version
            .drive_abci
            .validation_and_processing
            .has_nonce_validation
            != 0
        {
            return;
        }
        let result = identity_top_up_st()
            .has_identity_nonce_validation(platform_version)
            .expect("should not error");
        assert!(!result);
    }

    #[test]
    fn has_nonce_validation_v0_masternode_vote_returns_false() {
        let platform_version = PlatformVersion::first();
        if platform_version
            .drive_abci
            .validation_and_processing
            .has_nonce_validation
            != 0
        {
            return;
        }
        let result = masternode_vote_st()
            .has_identity_nonce_validation(platform_version)
            .expect("should not error");
        // In v0, MasternodeVote does NOT have nonce validation
        assert!(!result);
    }

    // ---- has_identity_nonce_validation with version 1 (PlatformVersion::latest) ----

    #[test]
    fn has_nonce_validation_v1_batch_returns_true() {
        let platform_version = PlatformVersion::latest();
        if platform_version
            .drive_abci
            .validation_and_processing
            .has_nonce_validation
            != 1
        {
            return;
        }
        let result = batch_st()
            .has_identity_nonce_validation(platform_version)
            .expect("should not error");
        assert!(result);
    }

    #[test]
    fn has_nonce_validation_v1_identity_update_returns_true() {
        let platform_version = PlatformVersion::latest();
        if platform_version
            .drive_abci
            .validation_and_processing
            .has_nonce_validation
            != 1
        {
            return;
        }
        let result = identity_update_st()
            .has_identity_nonce_validation(platform_version)
            .expect("should not error");
        assert!(result);
    }

    #[test]
    fn has_nonce_validation_v1_masternode_vote_returns_true() {
        let platform_version = PlatformVersion::latest();
        if platform_version
            .drive_abci
            .validation_and_processing
            .has_nonce_validation
            != 1
        {
            return;
        }
        let result = masternode_vote_st()
            .has_identity_nonce_validation(platform_version)
            .expect("should not error");
        // In v1, MasternodeVote DOES have nonce validation
        assert!(result);
    }

    #[test]
    fn has_nonce_validation_v1_identity_create_returns_false() {
        let platform_version = PlatformVersion::latest();
        if platform_version
            .drive_abci
            .validation_and_processing
            .has_nonce_validation
            != 1
        {
            return;
        }
        let result = identity_create_st()
            .has_identity_nonce_validation(platform_version)
            .expect("should not error");
        assert!(!result);
    }

    #[test]
    fn has_nonce_validation_v1_identity_top_up_returns_false() {
        let platform_version = PlatformVersion::latest();
        if platform_version
            .drive_abci
            .validation_and_processing
            .has_nonce_validation
            != 1
        {
            return;
        }
        let result = identity_top_up_st()
            .has_identity_nonce_validation(platform_version)
            .expect("should not error");
        assert!(!result);
    }

    // ---- unknown version returns error ----

    #[test]
    fn has_nonce_validation_unknown_version_returns_error() {
        let mut pv = PlatformVersion::latest().clone();
        pv.drive_abci.validation_and_processing.has_nonce_validation = 99;

        let result = batch_st().has_identity_nonce_validation(&pv);
        assert!(result.is_err());
        let err_string = format!("{:?}", result.unwrap_err());
        assert!(err_string.contains("UnknownVersionMismatch"));
    }
}
