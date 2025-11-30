use derive_more::From;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use std::collections::BTreeMap;

use dpp::prelude::{AddressNonce, UserFeeIncrease};

/// transformer module
pub mod transformer;
mod v0;

pub use v0::*;

/// bump address_input nonce action
#[derive(Debug, Clone, From)]
pub enum BumpAddressInputNoncesAction {
    /// v0
    V0(BumpAddressInputNoncesActionV0),
}

impl BumpAddressInputNonceActionAccessorsV0 for BumpAddressInputNoncesAction {
    fn inputs_with_remaining_balance(&self) -> &BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
        match self {
            BumpAddressInputNoncesAction::V0(v0) => &v0.inputs_with_remaining_balance,
        }
    }

    fn inputs_with_remaining_balance_and_outputs_owned(
        self,
    ) -> (
        BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        BTreeMap<PlatformAddress, Credits>,
    ) {
        match self {
            BumpAddressInputNoncesAction::V0(v0) => {
                (v0.inputs_with_remaining_balance, BTreeMap::new())
            }
        }
    }

    fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            BumpAddressInputNoncesAction::V0(v0) => v0.user_fee_increase,
        }
    }
}
