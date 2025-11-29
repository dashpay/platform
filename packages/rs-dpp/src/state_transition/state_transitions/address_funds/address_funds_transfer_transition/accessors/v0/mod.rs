use std::collections::BTreeMap;

use crate::address_funds::PlatformAddress;
use crate::fee::Credits;
use crate::prelude::KeyOfTypeNonce;

pub trait AddressFundsTransferTransitionAccessorsV0 {
    fn inputs(&self) -> &BTreeMap<PlatformAddress, (KeyOfTypeNonce, Credits)>;
    fn set_inputs(&mut self, inputs: BTreeMap<PlatformAddress, (KeyOfTypeNonce, Credits)>);
    fn outputs(&self) -> &BTreeMap<PlatformAddress, Credits>;
    fn set_outputs(&mut self, outputs: BTreeMap<PlatformAddress, Credits>);
}
