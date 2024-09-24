use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::identity::identity_credit_withdrawal::v0::IdentityCreditWithdrawalTransitionActionV0;
use crate::state_transition_action::identity::identity_credit_withdrawal::IdentityCreditWithdrawalTransitionAction;
use dpp::block::block_info::BlockInfo;
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use dpp::validation::ConsensusValidationResult;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl IdentityCreditWithdrawalTransitionAction {
    /// try from an identity credit withdrawal
    pub fn try_from_identity_credit_withdrawal(
        drive: &Drive,
        tx: TransactionArg,
        identity_credit_withdrawal: &IdentityCreditWithdrawalTransition,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Self>, Error> {
        match identity_credit_withdrawal {
            IdentityCreditWithdrawalTransition::V0(v0) => {
                Ok(ConsensusValidationResult::new_with_data(
                    IdentityCreditWithdrawalTransitionActionV0::from_identity_credit_withdrawal_v0(
                        v0,
                        block_info.time_ms,
                    )
                    .into(),
                ))
            }
            IdentityCreditWithdrawalTransition::V1(v1) => {
                IdentityCreditWithdrawalTransitionActionV0::try_from_identity_credit_withdrawal_v1(
                    drive,
                    tx,
                    v1,
                    block_info,
                    platform_version,
                )
                .map(|consensus_validation_result| {
                    consensus_validation_result.map(|withdrawal| withdrawal.into())
                })
            }
        }
    }
}
