use crate::prelude::KeyOfTypeNonce;
use std::collections::BTreeMap;

use crate::fee::Credits;
use crate::identity::KeyOfType;

pub trait AddressFundsTransferTransitionAccessorsV0 {
    fn inputs(&self) -> &BTreeMap<KeyOfType, (KeyOfTypeNonce, Credits)>;
    fn set_inputs(&mut self, inputs: BTreeMap<KeyOfType, (KeyOfTypeNonce, Credits)>);
    fn outputs(&self) -> &BTreeMap<KeyOfType, Credits>;
    fn set_outputs(&mut self, outputs: BTreeMap<KeyOfType, Credits>);
}
