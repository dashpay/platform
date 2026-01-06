/// transformer
pub mod transformer;

use dpp::address_funds::fee_strategy::AddressFundsFeeStrategy;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, UserFeeIncrease};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
/// Version 0 of the bump address input nonce action
/// This action is performed when we want to pay for validation of the state transition
/// but not execute it
pub struct BumpAddressInputNoncesActionV0 {
    /// inputs
    pub inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    /// fee strategy for how fees should be deducted
    pub fee_strategy: AddressFundsFeeStrategy,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}

/// document base transition action accessors v0
pub trait BumpAddressInputNonceActionAccessorsV0 {
    /// Get inputs
    fn inputs_with_remaining_balance(&self) -> &BTreeMap<PlatformAddress, (AddressNonce, Credits)>;

    /// Returns owned copies of inputs and outputs.
    fn inputs_with_remaining_balance_and_outputs_owned(
        self,
    ) -> (
        BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        BTreeMap<PlatformAddress, Credits>,
    );

    /// Get fee strategy
    fn fee_strategy(&self) -> &AddressFundsFeeStrategy;

    /// fee multiplier
    fn user_fee_increase(&self) -> UserFeeIncrease;
}
