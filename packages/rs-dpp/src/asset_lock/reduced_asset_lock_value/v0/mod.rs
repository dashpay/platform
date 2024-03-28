use crate::fee::Credits;
use bincode::{Decode, Encode};

#[derive(Debug, Clone, Encode, Decode, PartialEq)]
pub struct AssetLockValueV0 {
    pub(super) initial_credit_value: Credits,
    pub(super) tx_out_script: Vec<u8>,
    pub(super) remaining_credit_value: Credits,
}

pub trait AssetLockValueGettersV0 {
    fn initial_credit_value(&self) -> Credits;
    fn tx_out_script(&self) -> &Vec<u8>;

    fn tx_out_script_owned(self) -> Vec<u8>;
    fn remaining_credit_value(&self) -> Credits;
}

pub trait AssetLockValueSettersV0 {
    fn set_initial_credit_value(&mut self, value: Credits);
    fn set_tx_out_script(&mut self, value: Vec<u8>);
    fn set_remaining_credit_value(&mut self, value: Credits);
}
