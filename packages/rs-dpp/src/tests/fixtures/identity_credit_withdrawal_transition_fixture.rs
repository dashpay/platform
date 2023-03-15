use crate::prelude::Identifier;
use dashcore::{hashes::hex::FromHex, PubkeyHash, Script};
use platform_value::string_encoding::{encode, Encoding};
use platform_value::{platform_value, Value};
use serde_json::{json, Value as JsonValue};

use crate::{
    identity::state_transition::identity_credit_withdrawal_transition::Pooling,
    state_transition::StateTransitionType, version,
};

pub fn identity_credit_withdrawal_transition_fixture_raw_object() -> Value {
    platform_value!({
        "protocolVersion": version::LATEST_VERSION as u32,
        "type": StateTransitionType::IdentityCreditWithdrawal as u8,
        "identityId": Identifier::from([1_u8; 32]),
        "amount": 1042u64,
        "coreFeePerByte": 3u32,
        "pooling": Pooling::Never as u8,
        "outputScript": Script::new_p2pkh(&PubkeyHash::from_hex("0000000000000000000000000000000000000000").unwrap()).to_bytes(),
        "signature": vec![0_u8; 65],
        "signaturePublicKeyId": 0u32,
        "revision": 1u32,
    })
}

pub fn identity_credit_withdrawal_transition_fixture_json() -> JsonValue {
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
