use crate::fee::Credits;
use bincode::{Decode, Encode};
use platform_value::Bytes32;

#[derive(Debug, Clone, Encode, Decode, PartialEq)]
pub struct AssetLockValueV0 {
    pub(super) initial_credit_value: Credits,
    pub(super) tx_out_script: Vec<u8>,
    pub(super) remaining_credit_value: Credits,
    /// The used tags can represent any token of that we want to say has been used
    /// In practice for Platform this means that we are storing the asset lock transactions
    /// to prevent replay attacks.
    pub(super) used_tags: Vec<Bytes32>,
}

pub trait AssetLockValueGettersV0 {
    fn initial_credit_value(&self) -> Credits;
    fn tx_out_script(&self) -> &Vec<u8>;

    fn tx_out_script_owned(self) -> Vec<u8>;
    fn remaining_credit_value(&self) -> Credits;

    fn used_tags_ref(&self) -> &Vec<Bytes32>;
}

pub trait AssetLockValueSettersV0 {
    fn set_initial_credit_value(&mut self, value: Credits);
    fn set_tx_out_script(&mut self, value: Vec<u8>);
    fn set_remaining_credit_value(&mut self, value: Credits);

    fn set_used_tags(&mut self, tags: Vec<Bytes32>);
    fn add_used_tag(&mut self, tag: Bytes32);
}
