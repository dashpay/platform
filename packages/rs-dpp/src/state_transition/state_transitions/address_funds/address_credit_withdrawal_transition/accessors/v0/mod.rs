use std::collections::BTreeMap;

use crate::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use crate::fee::Credits;
use crate::identity::core_script::CoreScript;
use crate::prelude::AddressNonce;
use crate::withdrawal::Pooling;

pub trait AddressCreditWithdrawalTransitionAccessorsV0 {
    /// Get inputs
    fn inputs(&self) -> &BTreeMap<PlatformAddress, (AddressNonce, Credits)>;
    /// Set inputs
    fn set_inputs(&mut self, inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>);

    /// Get fee strategy
    fn fee_strategy(&self) -> &AddressFundsFeeStrategy;
    /// Set fee strategy
    fn set_fee_strategy(&mut self, fee_strategy: AddressFundsFeeStrategy);

    /// Get core fee per byte
    fn core_fee_per_byte(&self) -> u32;
    /// Set core fee per byte
    fn set_core_fee_per_byte(&mut self, core_fee_per_byte: u32);

    /// Get pooling
    fn pooling(&self) -> Pooling;
    /// Set pooling
    fn set_pooling(&mut self, pooling: Pooling);

    /// Get output script
    fn output_script(&self) -> &CoreScript;
    /// Set output script
    fn set_output_script(&mut self, output_script: CoreScript);
}
