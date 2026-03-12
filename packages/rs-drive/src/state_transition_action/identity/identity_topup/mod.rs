/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::identity::identity_topup::v0::IdentityTopUpTransitionActionV0;
use derive_more::From;
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;

use dpp::platform_value::{Bytes36, Identifier};
use dpp::prelude::UserFeeIncrease;

/// action
#[derive(Debug, Clone, From)]
pub enum IdentityTopUpTransitionAction {
    /// v0
    V0(IdentityTopUpTransitionActionV0),
}

impl IdentityTopUpTransitionAction {
    /// The balance being topped up
    pub fn top_up_asset_lock_value(&self) -> &AssetLockValue {
        match self {
            IdentityTopUpTransitionAction::V0(transition) => &transition.top_up_asset_lock_value,
        }
    }

    /// The balance being topped up
    pub fn top_up_asset_lock_value_consume(self) -> AssetLockValue {
        match self {
            IdentityTopUpTransitionAction::V0(transition) => transition.top_up_asset_lock_value,
        }
    }

    /// Identity Id
    pub fn identity_id(&self) -> Identifier {
        match self {
            IdentityTopUpTransitionAction::V0(transition) => transition.identity_id,
        }
    }

    /// Asset Lock Outpoint
    pub fn asset_lock_outpoint(&self) -> Bytes36 {
        match self {
            IdentityTopUpTransitionAction::V0(action) => action.asset_lock_outpoint,
        }
    }

    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            IdentityTopUpTransitionAction::V0(transition) => transition.user_fee_increase,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::identity::identity_topup::v0::IdentityTopUpTransitionActionV0;
    use dpp::asset_lock::reduced_asset_lock_value::{AssetLockValue, AssetLockValueGettersV0};
    use dpp::platform_value::{Bytes32, Bytes36};
    use dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;
    use dpp::version::PlatformVersion;

    fn make_asset_lock_value() -> AssetLockValue {
        let platform_version = PlatformVersion::latest();
        AssetLockValue::new(2000, vec![4, 5, 6], 1500, vec![], platform_version)
            .expect("expected asset lock value")
    }

    fn make_v0() -> IdentityTopUpTransitionActionV0 {
        IdentityTopUpTransitionActionV0 {
            signable_bytes_hasher: SignableBytesHasher::PreHashed(Bytes32([0xCC; 32])),
            top_up_asset_lock_value: make_asset_lock_value(),
            identity_id: Identifier::from([0xAA; 32]),
            asset_lock_outpoint: Bytes36([0xDD; 36]),
            user_fee_increase: 4,
        }
    }

    #[test]
    fn test_from_v0() {
        let v0 = make_v0();
        let action: IdentityTopUpTransitionAction = v0.into();
        assert!(matches!(action, IdentityTopUpTransitionAction::V0(_)));
    }

    #[test]
    fn test_top_up_asset_lock_value() {
        let action = IdentityTopUpTransitionAction::V0(make_v0());
        let alv = action.top_up_asset_lock_value();
        assert_eq!(alv.remaining_credit_value(), 1500);
        assert_eq!(alv.initial_credit_value(), 2000);
    }

    #[test]
    fn test_top_up_asset_lock_value_consume() {
        let action = IdentityTopUpTransitionAction::V0(make_v0());
        let alv = action.top_up_asset_lock_value_consume();
        assert_eq!(alv.remaining_credit_value(), 1500);
    }

    #[test]
    fn test_identity_id() {
        let action = IdentityTopUpTransitionAction::V0(make_v0());
        assert_eq!(action.identity_id(), Identifier::from([0xAA; 32]));
    }

    #[test]
    fn test_asset_lock_outpoint() {
        let action = IdentityTopUpTransitionAction::V0(make_v0());
        assert_eq!(action.asset_lock_outpoint(), Bytes36([0xDD; 36]));
    }

    #[test]
    fn test_user_fee_increase() {
        let action = IdentityTopUpTransitionAction::V0(make_v0());
        assert_eq!(action.user_fee_increase(), 4);
    }
}
