use dpp::{
    contracts::withdrawals_contract,
    data_contract::extra::common,
    prelude::{DataContract, Document, Identifier},
    util::{json_value::JsonValueExt, string_encoding::Encoding},
};
use grovedb::{Element, TransactionArg};
use serde_json::{json, Number, Value as JsonValue};

use crate::{
    drive::{block_info::BlockInfo, Drive},
    error::{drive::DriveError, Error},
    fee_pools::epochs::Epoch,
};

use super::paths::get_withdrawal_transactions_expired_ids_path_as_u8;

pub(crate) fn fetch_withdrawal_documents_by_status(
    drive: &Drive,
    status: u8,
    transaction: TransactionArg,
) -> Result<Vec<Document>, Error> {
    let query_value = json!({
        "where": [
            ["status", "==", status],
        ],
        "orderBy": [
            ["$createdAt", "desc"],
        ]
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let (documents, _, _) = drive.query_documents(
        &query_cbor,
        Identifier::from_string(
            &withdrawals_contract::system_ids().contract_id,
            Encoding::Base58,
        )
        .map_err(|_| {
            Error::Drive(DriveError::CorruptedCodeExecution(
                "Can't create withdrawals id identifier from string",
            ))
        })?
        .to_buffer(),
        withdrawals_contract::types::WITHDRAWAL,
        None,
        transaction,
    )?;

    let documents = documents
        .into_iter()
        .map(|document_cbor| {
            Document::from_cbor(document_cbor).map_err(|_| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "Can't create a document from cbor",
                ))
            })
        })
        .collect::<Result<Vec<Document>, Error>>()?;

    Ok(documents)
}

pub(crate) fn update_document_data<F>(
    drive: &Drive,
    contract: &DataContract,
    document: &mut Document,
    block_time_ms: u64,
    block_height: u64,
    current_epoch_index: u16,
    transaction: TransactionArg,
    update_fn: F,
) -> Result<(), Error>
where
    F: Fn(&mut Document) -> Result<&mut Document, Error>,
{
    let document = update_fn(document)?;

    drive.update_document_for_contract_cbor(
        &document.to_cbor().map_err(|_| {
            Error::Drive(DriveError::CorruptedCodeExecution(
                "Can't cbor withdrawal document",
            ))
        })?,
        &contract.to_cbor().map_err(|_| {
            Error::Drive(DriveError::CorruptedCodeExecution(
                "Can't cbor withdrawal data contract",
            ))
        })?,
        withdrawals_contract::types::WITHDRAWAL,
        Some(document.owner_id.to_buffer()),
        BlockInfo {
            time_ms: block_time_ms,
            height: block_height,
            epoch: Epoch::new(current_epoch_index),
        },
        true,
        None,
        transaction,
    )?;

    Ok(())
}

fn fetch_core_block_transactions(
    drive: &Drive,
    last_synced_core_height: u64,
    core_chain_locked_height: u64,
) -> Result<Vec<String>, Error> {
    let core_rpc =
        drive
            .core_rpc
            .as_ref()
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "Core RPC client has not been set up",
            )))?;

    let mut tx_hashes: Vec<String> = vec![];

    for height in last_synced_core_height..=core_chain_locked_height {
        let block_hash = core_rpc.get_block_hash(height).map_err(|_| {
            Error::Drive(DriveError::CorruptedCodeExecution(
                "could not get block by height",
            ))
        })?;

        let block_json: JsonValue = core_rpc.get_block_json(&block_hash).map_err(|_| {
            Error::Drive(DriveError::CorruptedCodeExecution(
                "could not get block by hash",
            ))
        })?;

        if let Some(transactions) = block_json.get("tx") {
            if let Some(transactions) = transactions.as_array() {
                for transaction_hash in transactions {
                    tx_hashes.push(
                        transaction_hash
                            .as_str()
                            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                                "could not get transaction hash as string",
                            )))?
                            .to_string(),
                    );
                }
            }
        }
    }

    Ok(tx_hashes)
}

impl Drive {
    /// Update statuses for pending withdrawals
    pub fn update_withdrawal_statuses(
        &self,
        last_synced_core_height: u64,
        core_chain_locked_height: u64,
        block_time_ms: u64,
        block_height: u64,
        current_epoch_index: u16,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        let maybe_data_contract = self.get_cached_contract_with_fetch_info(
            Identifier::from_string(
                &withdrawals_contract::system_ids().contract_id,
                Encoding::Base58,
            )
            .map_err(|_| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "Can't create withdrawals id identifier from string",
                ))
            })?
            .to_buffer(),
            transaction,
        );

        let contract_fetch_info = maybe_data_contract.ok_or(Error::Drive(
            DriveError::CorruptedCodeExecution("Can't fetch withdrawal data contract"),
        ))?;

        let core_transactions =
            fetch_core_block_transactions(self, last_synced_core_height, core_chain_locked_height)?;

        let broadcasted_documents = fetch_withdrawal_documents_by_status(
            self,
            withdrawals_contract::statuses::BROADCASTED,
            transaction,
        )?;

        for mut document in broadcasted_documents {
            let transaction_sign_height =
                document
                    .data
                    .get_u64("transactionSignHeight")
                    .map_err(|_| {
                        Error::Drive(DriveError::CorruptedCodeExecution(
                            "Can't get transactionSignHeight from withdrawal document",
                        ))
                    })?;

            let transaction_id_bytes = document.data.get_bytes("transactionId").map_err(|_| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "Can't get transactionId from withdrawal document",
                ))
            })?;

            let transaction_id = hex::encode(transaction_id_bytes);

            let transaction_index = document.data.get_u64("transactionIdex").map_err(|_| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "Can't get transactionIdex from withdrawal document",
                ))
            })?;

            if core_transactions.contains(&transaction_id)
                || core_chain_locked_height - transaction_sign_height > 48
            {
                let status = if core_transactions.contains(&transaction_id) {
                    withdrawals_contract::statuses::COMPLETE
                } else {
                    let bytes = transaction_index.to_be_bytes();

                    let path = get_withdrawal_transactions_expired_ids_path_as_u8();

                    self.grove
                        .insert(
                            path,
                            &bytes,
                            Element::Item(bytes.to_vec(), None),
                            None,
                            transaction,
                        )
                        .unwrap()?;

                    withdrawals_contract::statuses::EXPIRED
                };

                update_document_data(
                    self,
                    &contract_fetch_info.contract,
                    &mut document,
                    block_time_ms,
                    block_height,
                    current_epoch_index,
                    transaction,
                    |document: &mut Document| -> Result<&mut Document, Error> {
                        document
                            .data
                            .insert(
                                "status".to_string(),
                                JsonValue::Number(Number::from(status)),
                            )
                            .map_err(|_| {
                                Error::Drive(DriveError::CorruptedCodeExecution(
                                    "Can't update document field: status",
                                ))
                            })?;

                        document.revision += 1;

                        Ok(document)
                    },
                )?;
            }
        }

        Ok(())
    }
}
