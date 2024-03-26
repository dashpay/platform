use crate::fee::Credits;
use bincode::{Decode, Encode};

#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq)]
pub struct AssetLockValueV0 {
    pub(super) initial_credit_value: Credits,
    pub(super) remaining_credit_value: Credits,
}

pub trait AssetLockValueGettersV0 {
    fn initial_credit_value(&self) -> Credits;
    fn remaining_credit_value(&self) -> Credits;
}

pub trait AssetLockValueSettersV0 {
    fn set_initial_credit_value(&mut self, value: Credits);
    fn set_remaining_credit_value(&mut self, value: Credits);
}
