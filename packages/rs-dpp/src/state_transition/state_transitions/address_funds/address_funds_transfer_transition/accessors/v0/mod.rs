use crate::prelude::{KeyOfTypeNonce, UserFeeIncrease};
use std::collections::BTreeMap;

use crate::fee::Credits;
use crate::identity::KeyOfType;

pub trait AddressFundsTransferTransitionAccessorsV0 {
    fn inputs(&self) -> &BTreeMap<KeyOfType, (KeyOfTypeNonce, Credits)>;
    fn set_inputs(&mut self, inputs: BTreeMap<KeyOfType, (KeyOfTypeNonce, Credits)>);
    fn outputs(&self) -> &BTreeMap<KeyOfType, Credits>;
    fn set_outputs(&mut self, outputs: BTreeMap<KeyOfType, Credits>);
    fn user_fee_increase(&self) -> UserFeeIncrease;
    fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease);
}
