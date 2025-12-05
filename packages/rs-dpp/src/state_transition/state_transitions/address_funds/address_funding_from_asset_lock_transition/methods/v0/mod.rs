#[cfg(feature = "state-transition-signing")]
use std::collections::BTreeMap;

#[cfg(feature = "state-transition-signing")]
use crate::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::{AddressNonce, AssetLockProof};
use crate::state_transition::StateTransitionType;
#[cfg(feature = "state-transition-signing")]
use crate::{prelude::UserFeeIncrease, state_transition::StateTransition, ProtocolError};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

pub trait AddressFundingFromAssetLockTransitionMethodsV0 {
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn try_from_asset_lock_with_signer<S: Signer<PlatformAddress>>(
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        outputs: BTreeMap<PlatformAddress, Option<Credits>>,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError>;

    /// Get State Transition Type
    fn get_type() -> StateTransitionType {
        StateTransitionType::AddressFundingFromAssetLock
    }
}
