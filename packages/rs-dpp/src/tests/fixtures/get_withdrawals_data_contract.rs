use crate::contracts::withdrawals_contract;
use lazy_static::lazy_static;
use serde_json::Value;

use crate::prelude::*;

use crate::util::serializer::value_to_cbor;

lazy_static! {
    static ref WITHDRAWALS_CONTRACT_DOCUMENTS: Value = serde_json::from_str(include_str!(
        "./../../../contracts/withdrawals/withdrawals-contract-documents.json"
    ))
    .expect("withdrawals contract should be valid json");
    static ref WITHDRAWALS_CONTRACT_SCHEMA: Value = serde_json::from_str(include_str!(
        "./../../../contracts/withdrawals/withdrawals-contract.json"
    ))
    .expect("withdrawals contract should be valid json");
}

pub fn get_withdrawals_data_contract_fixture(owner_id: Option<Identifier>) -> DataContract {
    let withdrawal_cbor = value_to_cbor(WITHDRAWALS_CONTRACT_SCHEMA.clone(), Some(1))
        .expect("should convert json to cbor value");

    let mut data_contract = DataContract::from_cbor(withdrawal_cbor)
        .expect("expected to deserialize withdrawal contract");

    if let Some(owner_id) = owner_id {
        data_contract.owner_id = owner_id;
    }

    data_contract.id = *withdrawals_contract::CONTRACT_ID;

    data_contract
}
