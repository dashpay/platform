/// transformer
pub mod transformer;

use dpp::fee::Credits;
use dpp::identity::KeyOfType;
use dpp::prelude::{AddressNonce, UserFeeIncrease};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
/// Version 0 of the bump address input nonce action
/// This action is performed when we want to pay for validation of the state transition
/// but not execute it
pub struct BumpAddressInputNoncesActionV0 {
    /// inputs
    pub inputs_with_remaining_balance: BTreeMap<KeyOfType, (AddressNonce, Credits)>,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}

/// document base transition action accessors v0
pub trait BumpAddressInputNonceActionAccessorsV0 {
    /// Get inputs
    fn inputs_with_remaining_balance(&self) -> &BTreeMap<KeyOfType, (AddressNonce, Credits)>;

    /// Returns owned copies of inputs and outputs.
    fn inputs_with_remaining_balance_and_outputs_owned(
        self,
    ) -> (
        BTreeMap<KeyOfType, (AddressNonce, Credits)>,
        BTreeMap<KeyOfType, Credits>,
    );

    /// fee multiplier
    fn user_fee_increase(&self) -> UserFeeIncrease;
}
