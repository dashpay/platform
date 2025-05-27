use crate::data_contract::TokenContractPosition;
use crate::prelude::{
    DerivationEncryptionKeyIndex, RecipientKeyIndex, RootEncryptionKeyIndex, SenderKeyIndex,
};
use crate::util::hash::hash_double;

pub mod allowed_currency;
pub mod contract_info;
pub mod emergency_action;
pub mod errors;
pub mod gas_fees_paid_by;
pub mod info;
pub mod status;
pub mod token_amount_on_contract_token;
pub mod token_event;
pub mod token_payment_info;
pub mod token_pricing_schedule;

pub const MAX_TOKEN_NOTE_LEN: usize = 2048;
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub type SharedEncryptedNote = (SenderKeyIndex, RecipientKeyIndex, Vec<u8>);
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub type PrivateEncryptedNote = (
    RootEncryptionKeyIndex,
    DerivationEncryptionKeyIndex,
    Vec<u8>,
);

pub fn calculate_token_id(contract_id: &[u8; 32], token_pos: TokenContractPosition) -> [u8; 32] {
    let mut bytes = b"dash_token".to_vec();
    bytes.extend_from_slice(contract_id);
    bytes.extend_from_slice(&token_pos.to_be_bytes());
    hash_double(bytes)
}
