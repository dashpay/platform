/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::contract::data_contract_update::v0::DataContractUpdateTransitionActionV0;
use derive_more::From;
use dpp::data_contract::DataContract;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};

/// data contract update transition action
#[derive(Debug, Clone, From)]
pub enum DataContractUpdateTransitionAction {
    /// v0
    V0(DataContractUpdateTransitionActionV0),
}

impl DataContractUpdateTransitionAction {
    /// data contract
    pub fn data_contract(self) -> DataContract {
        match self {
            DataContractUpdateTransitionAction::V0(transition) => transition.data_contract,
        }
    }
    /// data contract ref
    pub fn data_contract_ref(&self) -> &DataContract {
        match self {
            DataContractUpdateTransitionAction::V0(transition) => &transition.data_contract,
        }
    }

    /// data contract mut
    pub fn data_contract_mut(&mut self) -> &mut DataContract {
        match self {
            DataContractUpdateTransitionAction::V0(transition) => &mut transition.data_contract,
        }
    }

    /// identity contract nonce
    pub fn identity_contract_nonce(&self) -> IdentityNonce {
        match self {
            DataContractUpdateTransitionAction::V0(transition) => {
                transition.identity_contract_nonce
            }
        }
    }

    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            DataContractUpdateTransitionAction::V0(transition) => transition.user_fee_increase,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use platform_version::version::PlatformVersion;

    fn make_action() -> DataContractUpdateTransitionAction {
        let fixture =
            get_data_contract_fixture(None, 0, PlatformVersion::latest().protocol_version);
        let dc = fixture.data_contract_owned();
        let v0 = DataContractUpdateTransitionActionV0 {
            data_contract: dc,
            identity_contract_nonce: 99,
            user_fee_increase: 3,
        };
        DataContractUpdateTransitionAction::from(v0)
    }

    #[test]
    fn test_from_v0() {
        let action = make_action();
        assert!(matches!(
            action,
            DataContractUpdateTransitionAction::V0(_)
        ));
    }

    #[test]
    fn test_data_contract_ref() {
        let action = make_action();
        let dc_ref = action.data_contract_ref();
        let _id = dc_ref.id();
    }

    #[test]
    fn test_data_contract_owned() {
        let action = make_action();
        let original_id = action.data_contract_ref().id();
        let dc = action.data_contract();
        assert_eq!(dc.id(), original_id);
    }

    #[test]
    fn test_data_contract_mut() {
        let mut action = make_action();
        let dc_mut = action.data_contract_mut();
        // Verify we can obtain a mutable reference without panic.
        let _id = dc_mut.id();
    }

    #[test]
    fn test_identity_contract_nonce() {
        let action = make_action();
        assert_eq!(action.identity_contract_nonce(), 99);
    }

    #[test]
    fn test_user_fee_increase() {
        let action = make_action();
        assert_eq!(action.user_fee_increase(), 3);
    }

    #[test]
    fn test_clone() {
        let action = make_action();
        let cloned = action.clone();
        assert_eq!(cloned.identity_contract_nonce(), 99);
        assert_eq!(cloned.user_fee_increase(), 3);
    }

    #[test]
    fn test_debug() {
        let action = make_action();
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("V0"));
    }
}
