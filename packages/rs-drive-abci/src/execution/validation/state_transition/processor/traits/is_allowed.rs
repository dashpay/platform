use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::consensus::basic::state_transition::StateTransitionNotActiveError;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::StateTransition;
use dpp::version::feature_initial_protocol_versions::ADDRESS_FUNDS_INITIAL_PROTOCOL_VERSION;
use dpp::version::PlatformVersion;

/// A trait for validating state transitions within a blockchain.
pub(crate) trait StateTransitionIsAllowedValidationV0 {
    /// This means we should validate is state transition is allowed
    fn has_is_allowed_validation(&self) -> Result<bool, Error>;
    /// Preliminary validation for a state transition
    fn validate_is_allowed<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<()>, Error>;
}

impl StateTransitionIsAllowedValidationV0 for StateTransition {
    fn has_is_allowed_validation(&self) -> Result<bool, Error> {
        match self {
            StateTransition::Batch(_)
            | StateTransition::IdentityTopUpFromAddresses(_)
            | StateTransition::IdentityCreateFromAddresses(_)
            | StateTransition::AddressFundsTransfer(_)
            | StateTransition::IdentityCreditTransferToAddresses(_)
            | StateTransition::AddressFundingFromAssetLock(_)
            | StateTransition::AddressCreditWithdrawal(_) => Ok(true),
            StateTransition::DataContractCreate(_)
            | StateTransition::DataContractUpdate(_)
            | StateTransition::IdentityCreate(_)
            | StateTransition::IdentityTopUp(_)
            | StateTransition::IdentityCreditWithdrawal(_)
            | StateTransition::IdentityUpdate(_)
            | StateTransition::IdentityCreditTransfer(_)
            | StateTransition::MasternodeVote(_) => Ok(false),
            StateTransition::Shield(_)
            | StateTransition::ShieldedTransfer(_)
            | StateTransition::Unshield(_)
            | StateTransition::ShieldFromAssetLock(_)
            | StateTransition::ShieldedWithdrawal(_) => {
                todo!("shielded transitions not yet implemented")
            }
        }
    }

    fn validate_is_allowed<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<()>, Error> {
        match self {
            StateTransition::Batch(st) => st.validate_is_allowed(platform, platform_version),
            StateTransition::IdentityTopUpFromAddresses(_)
            | StateTransition::IdentityCreateFromAddresses(_)
            | StateTransition::AddressFundsTransfer(_)
            | StateTransition::IdentityCreditTransferToAddresses(_)
            | StateTransition::AddressFundingFromAssetLock(_)
            | StateTransition::AddressCreditWithdrawal(_) => {
                if platform_version.protocol_version >= ADDRESS_FUNDS_INITIAL_PROTOCOL_VERSION {
                    Ok(ConsensusValidationResult::new())
                } else {
                    Ok(ConsensusValidationResult::new_with_errors(vec![
                        StateTransitionNotActiveError::new(
                            self.state_transition_type().to_string(),
                            platform_version.protocol_version,
                            ADDRESS_FUNDS_INITIAL_PROTOCOL_VERSION,
                        )
                        .into(),
                    ]))
                }
            }
            _ => Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "validate_is_allowed is not implemented for this state transition",
            ))),
        }
    }
}
