use crate::fee::Credits;
use bincode::{Decode, Encode};

#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq)]
pub struct ReducedAssetLockValueV0 {
    pub(super) initial_credit_value: Credits,
    pub(super) remaining_credit_value: Credits,
}

pub trait ReducedAssetLockValueGettersV0 {
    fn initial_credit_value(&self) -> Option<Credits>;
    fn remaining_credit_value(&self) -> Option<Credits>;
}

pub trait ReducedAssetLockValueSettersV0 {
    fn set_initial_credit_value(&mut self, value: Credits);
    fn set_remaining_credit_value(&mut self, value: Credits);
}
