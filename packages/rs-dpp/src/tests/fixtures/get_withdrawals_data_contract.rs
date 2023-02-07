use std::sync::Arc;

use lazy_static::lazy_static;
use serde_json::Value;

use crate::prelude::*;

use crate::data_contract::extra::common::json_document_to_cbor;

lazy_static! {
    static ref WITHDRAWALS_SCHEMA: Value = serde_json::from_str(include_str!(
        "./../../../contracts/withdrawals/withdrawals-contract-documents.json"
    ))
    .expect("withdrawals contract should be valid json");
}

pub fn get_withdrawals_data_contract_fixture(owner_id: Option<Identifier>) -> DataContract {
    let withdrawal_contract_path = "./contracts/withdrawals/withdrawals-contract-documents.json";

    let withdrawal_cbor = json_document_to_cbor(withdrawal_contract_path, Some(1))
        .expect("expected to get cbor document");

    let mut data_contract = DataContract::from_cbor(withdrawal_cbor)
        .expect("expected to deserialize withdrawal contract");

    if let Some(owner_id) = owner_id {
        data_contract.owner_id = owner_id;
    }
    data_contract
}
