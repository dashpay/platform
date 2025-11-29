use std::collections::BTreeMap;

use crate::address_funds::PlatformAddress;
use crate::fee::Credits;
use crate::prelude::AddressNonce;

pub trait AddressFundsTransferTransitionAccessorsV0 {
    fn inputs(&self) -> &BTreeMap<PlatformAddress, (AddressNonce, Credits)>;
    fn set_inputs(&mut self, inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>);
    fn outputs(&self) -> &BTreeMap<PlatformAddress, Credits>;
    fn set_outputs(&mut self, outputs: BTreeMap<PlatformAddress, Credits>);
}
