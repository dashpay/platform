use super::v0::AddressCreditWithdrawalStateTransitionTransformIntoActionValidationV0;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::state_transitions::stamp_withdrawal_document;
use dpp::address_funds::PlatformAddress;
use dpp::block::block_info::BlockInfo;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, ConsensusValidationResult};
use dpp::state_transition::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
use dpp::version::PlatformVersion;
use drive::state_transition_action::address_funds::address_credit_withdrawal::AddressCreditWithdrawalTransitionAction;
use drive::state_transition_action::StateTransitionAction;
use std::collections::BTreeMap;

pub(in crate::execution::validation::state_transition::state_transitions::address_credit_withdrawal) trait AddressCreditWithdrawalStateTransitionTransformIntoActionValidationV1
{
    fn transform_into_action_v1(
        &self,
        block_info: &BlockInfo,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl AddressCreditWithdrawalStateTransitionTransformIntoActionValidationV1
    for AddressCreditWithdrawalTransition
{
    fn transform_into_action_v1(
        &self,
        block_info: &BlockInfo,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        self.transform_into_action_v0(block_info, inputs_with_remaining_balance, platform_version)
            .and_then(|result| {
                result.map_result(|mut action| {
                    match &mut action {
                        StateTransitionAction::AddressCreditWithdrawal(
                            AddressCreditWithdrawalTransitionAction::V0(withdrawal),
                        ) => stamp_withdrawal_document(
                            &mut withdrawal.prepared_withdrawal_document,
                            platform_version,
                        ),
                        _ => {
                            return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                                "address withdrawal transformer returned an unrelated action",
                            )));
                        }
                    }
                    Ok(action)
                })
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::address_funds::AddressFundsFeeStrategyStep;
    use dpp::document::Document;
    use dpp::identity::core_script::CoreScript;
    use dpp::state_transition::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;
    use dpp::withdrawal::Pooling;

    fn transition() -> AddressCreditWithdrawalTransition {
        AddressCreditWithdrawalTransition::V0(AddressCreditWithdrawalTransitionV0 {
            inputs: BTreeMap::from([(PlatformAddress::P2pkh([1; 20]), (0, 2_000_000))]),
            output: None,
            fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            core_fee_per_byte: 1,
            pooling: Pooling::Never,
            output_script: CoreScript::new_p2pkh([5; 20]),
            user_fee_increase: 0,
            input_witnesses: vec![],
        })
    }

    fn contract_version(result: ConsensusValidationResult<StateTransitionAction>) -> Option<u32> {
        match result.data.expect("withdrawal action") {
            StateTransitionAction::AddressCreditWithdrawal(
                AddressCreditWithdrawalTransitionAction::V0(withdrawal),
            ) => match withdrawal.prepared_withdrawal_document {
                Document::V0(document) => document.contract_version,
            },
            _ => panic!("expected address withdrawal action"),
        }
    }

    #[test]
    fn should_stamp_only_the_v1_withdrawal_document() {
        let transition = transition();
        let remaining = BTreeMap::from([(PlatformAddress::P2pkh([1; 20]), (1, 0))]);

        assert_eq!(
            contract_version(
                transition
                    .transform_into_action_v0(
                        &BlockInfo::default(),
                        remaining.clone(),
                        PlatformVersion::get(13).expect("platform version 13"),
                    )
                    .expect("v0 transform"),
            ),
            None
        );
        assert_eq!(
            contract_version(
                transition
                    .transform_into_action_v1(
                        &BlockInfo::default(),
                        remaining,
                        PlatformVersion::latest(),
                    )
                    .expect("v1 transform"),
            ),
            Some(PlatformVersion::latest().system_data_contracts.withdrawals as u32)
        );
    }
}
