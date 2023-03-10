use dashcore::{hashes::hex::FromHex, PubkeyHash, Script};
use serde_json::{json, Value};
use platform_value::string_encoding::{encode, Encoding};

use crate::{
    identity::state_transition::identity_credit_withdrawal_transition::Pooling,
    state_transition::StateTransitionType,
    version,
};

pub fn identity_credit_withdrawal_transition_fixture_raw_object() -> Value {
    json!({
        "protocolVersion": version::LATEST_VERSION,
        "type": StateTransitionType::IdentityCreditWithdrawal,
        "identityId": vec![1_u8; 32],
        "amount": 1042,
        "coreFeePerByte": 3,
        "pooling": Pooling::Never,
        "outputScript": Script::new_p2pkh(&PubkeyHash::from_hex("0000000000000000000000000000000000000000").unwrap()).to_bytes(),
        "signature": vec![0_u8; 65],
        "signaturePublicKeyId": 0,
        "revision": 1,
    })
}

pub fn identity_credit_withdrawal_transition_fixture_json() -> Value {
    json!({
        "protocolVersion": version::LATEST_VERSION,
        "type": StateTransitionType::IdentityCreditWithdrawal,
        "identityId": encode(&[1_u8; 32], Encoding::Base58),
        "amount": 1042,
        "coreFeePerByte": 3,
        "pooling": Pooling::Never,
        "outputScript": encode(&Script::new_p2pkh(&PubkeyHash::from_hex("0000000000000000000000000000000000000000").unwrap()).to_bytes(), Encoding::Base64),
        "signature": encode(&[0_u8; 65], Encoding::Base64),
        "signaturePublicKeyId": 0,
        "revision": 1,
    })
}
