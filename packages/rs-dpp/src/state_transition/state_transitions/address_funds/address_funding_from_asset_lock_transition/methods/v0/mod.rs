#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::identity::KeyOfType;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::AssetLockProof;
use crate::state_transition::StateTransitionType;
#[cfg(feature = "state-transition-signing")]
use crate::{prelude::UserFeeIncrease, state_transition::StateTransition, ProtocolError};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;
#[cfg(feature = "state-transition-signing")]
use std::collections::BTreeMap;

pub trait AddressFundingFromAssetLockTransitionMethodsV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_asset_lock(
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        outputs: BTreeMap<KeyOfType, Credits>,
        output_paying_fees: u16,
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError>;

    /// Get State Transition Type
    fn get_type() -> StateTransitionType {
        StateTransitionType::AddressFundingFromAssetLock
    }
}
