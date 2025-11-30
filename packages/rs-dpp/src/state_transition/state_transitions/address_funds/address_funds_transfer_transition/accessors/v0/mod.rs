use std::collections::BTreeMap;

use crate::address_funds::PlatformAddress;
use crate::fee::Credits;

pub trait AddressFundsTransferTransitionAccessorsV0 {
    fn outputs(&self) -> &BTreeMap<PlatformAddress, Credits>;
    fn set_outputs(&mut self, outputs: BTreeMap<PlatformAddress, Credits>);
}
