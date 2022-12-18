use std::collections::HashMap;

use dashcore::{
    blockdata::transaction::special_transaction::asset_unlock::unqualified_asset_unlock::{
        AssetUnlockBasePayload, AssetUnlockBaseTransactionInfo,
    },
    consensus::Encodable,
    Script, TxOut,
};
use dpp::{
    contracts::withdrawals_contract,
    identity::convert_credits_to_satoshi,
    prelude::{Document, Identifier},
    util::{hash, json_value::JsonValueExt, string_encoding::Encoding},
};
use grovedb::TransactionArg;
use serde_json::{Number, Value as JsonValue};

use crate::{
    drive::{batch::GroveDbOpBatch, Drive},
    error::{drive::DriveError, Error},
};

use super::{
    paths::WithdrawalTransaction,
    withdrawal_status::{fetch_withdrawal_documents_by_status, update_document_data},
};

fn build_withdrawal_transactions_from_documents(
    drive: &Drive,
    documents: &[Document],
    transaction: TransactionArg,
) -> Result<HashMap<Identifier, WithdrawalTransaction>, Error> {
    let mut withdrawals: HashMap<Identifier, WithdrawalTransaction> = HashMap::new();

    let latest_withdrawal_index = drive.fetch_latest_withdrawal_transaction_index(transaction)?;

    for (i, document) in documents.iter().enumerate() {
        let output_script = document.data.get_bytes("outputScript").map_err(|_| {
            Error::Drive(DriveError::CorruptedCodeExecution(
                "Can't get outputScript from withdrawal document",
            ))
        })?;

        let amount = document.data.get_u64("amount").map_err(|_| {
            Error::Drive(DriveError::CorruptedCodeExecution(
                "Can't get amount from withdrawal document",
            ))
        })?;

        let core_fee_per_byte = document.data.get_u64("coreFeePerByte").map_err(|_| {
            Error::Drive(DriveError::CorruptedCodeExecution(
                "Can't get coreFeePerByte from withdrawal document",
            ))
        })?;

        let state_transition_size = 184;

        let output_script: Script = Script::from(output_script);

        let tx_out = TxOut {
            value: convert_credits_to_satoshi(amount),
            script_pubkey: output_script,
        };

        let transaction_index = latest_withdrawal_index + i as u64;

        let withdrawal_transaction = AssetUnlockBaseTransactionInfo {
            version: 1,
            lock_time: 0,
            output: vec![tx_out],
            base_payload: AssetUnlockBasePayload {
                version: 1,
                index: transaction_index,
                fee: (state_transition_size * core_fee_per_byte * 1000) as u32,
            },
        };

        let mut transaction_buffer: Vec<u8> = vec![];

        withdrawal_transaction
            .consensus_encode(&mut transaction_buffer)
            .map_err(|_| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "Can't consensus encode a withdrawal transaction",
                ))
            })?;

        withdrawals.insert(
            document.id.clone(),
            (
                transaction_index.to_be_bytes().to_vec(),
                transaction_buffer.clone(),
            ),
        );
    }

    Ok(withdrawals)
}

impl Drive {
    /// Pool withdrawal documents into transactions
    pub fn pool_withdrawals_into_transactions(
        &self,
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

        let mut documents = fetch_withdrawal_documents_by_status(
            self,
            withdrawals_contract::statuses::QUEUED,
            transaction,
        )?;

        let withdrawal_transactions =
            build_withdrawal_transactions_from_documents(self, &documents, transaction)?;

        for document in documents.iter_mut() {
            let transaction_id =
                hash::hash(withdrawal_transactions.get(&document.id).unwrap().1.clone());

            update_document_data(
                self,
                &contract_fetch_info.contract,
                document,
                block_time_ms,
                block_height,
                current_epoch_index,
                transaction,
                |document: &mut Document| -> Result<&mut Document, Error> {
                    document
                        .data
                        .insert(
                            "transactionId".to_string(),
                            JsonValue::Array(
                                transaction_id
                                    .clone()
                                    .into_iter()
                                    .map(|byte| JsonValue::Number(Number::from(byte)))
                                    .collect(),
                            ),
                        )
                        .map_err(|_| {
                            Error::Drive(DriveError::CorruptedCodeExecution(
                                "Can't update document field: transactionId",
                            ))
                        })?;

                    document
                        .data
                        .insert(
                            "status".to_string(),
                            JsonValue::Number(Number::from(withdrawals_contract::statuses::POOLED)),
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

        let mut batch = GroveDbOpBatch::new();

        let withdrawal_transactions = withdrawal_transactions
            .values()
            .into_iter()
            .cloned()
            .collect();

        self.add_enqueue_withdrawal_transaction_operations(&mut batch, withdrawal_transactions);

        if !batch.is_empty() {
            self.grove_apply_batch(batch, true, transaction)?;
        }

        Ok(())
    }
}
