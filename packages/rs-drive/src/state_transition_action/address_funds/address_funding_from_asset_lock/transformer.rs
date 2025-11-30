use crate::state_transition_action::address_funds::address_funding_from_asset_lock::v0::AddressFundingFromAssetLockTransitionActionV0;
use crate::state_transition_action::address_funds::address_funding_from_asset_lock::AddressFundingFromAssetLockTransitionAction;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
use std::collections::BTreeMap;

impl AddressFundingFromAssetLockTransitionAction {
    /// Transforms the state transition into an action by validating inputs against provided balances.
    pub fn try_from_transition(
        value: &AddressFundingFromAssetLockTransition,
        input_balances: BTreeMap<PlatformAddress, Credits>,
    ) -> ConsensusValidationResult<Self> {
        match value {
            AddressFundingFromAssetLockTransition::V0(v0) => {
                let result = AddressFundingFromAssetLockTransitionActionV0::try_from_transition(
                    v0,
                    input_balances,
                );
                result.map(|action| action.into())
            }
        }
    }
}
