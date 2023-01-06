// use std::collections::HashMap;

// use dashcore::{
//     blockdata::transaction::special_transaction::asset_unlock::unqualified_asset_unlock::{
//         AssetUnlockBasePayload, AssetUnlockBaseTransactionInfo,
//     },
//     consensus::Encodable,
//     Script, TxOut,
// };
// use dpp::{
//     identity::convert_credits_to_satoshi,
//     prelude::{Document, Identifier},
//     util::json_value::JsonValueExt,
// };
// use grovedb::TransactionArg;

// use crate::{
//     drive::Drive,
//     error::{drive::DriveError, Error},
// };

// use super::paths::WithdrawalTransaction;

// impl Drive {}

// #[cfg(test)]
// mod tests {
//     use crate::common::helpers::setup::setup_drive_with_initial_state_structure;

//     use serde_json::json;

//     use dpp::{
//         contracts::withdrawals_contract,
//         tests::fixtures::{get_withdrawal_document_fixture, get_withdrawals_data_contract_fixture},
//     };

//     use crate::{
//         common::helpers::setup::{setup_document, setup_system_data_contract},
//         drive::identity::withdrawals::paths::WithdrawalTransaction,
//     };

// }
