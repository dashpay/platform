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
    /// Update statuses for broadcasted withdrawals
    pub fn update_broadcasted_withdrawal_transaction_statuses(
        &self,
        last_synced_core_height: u64,
        core_chain_locked_height: u64,
        block_time_ms: u64,
        block_height: u64,
        current_epoch_index: u16,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        let (_, maybe_data_contract) = self.get_contract_with_fetch_info(
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
            Some(&Epoch::new(current_epoch_index)),
            transaction,
        )?;

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

            let transaction_index = document.data.get_u64("transactionIndex").map_err(|_| {
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

#[cfg(test)]
mod tests {
    use crate::drive::identity::withdrawals::withdrawal_status::fetch_withdrawal_documents_by_status;
    use dpp::contracts::withdrawals_contract;
    use dpp::tests::fixtures::get_withdrawal_document_fixture;
    use dpp::tests::fixtures::get_withdrawals_data_contract_fixture;
    use serde_json::json;

    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;
    use crate::common::helpers::setup::{setup_document, setup_system_data_contract};

    use dashcore::hashes::hex::ToHex;

    use crate::rpc::core::MockCoreRPCLike;

    use dashcore::{hashes::hex::FromHex, BlockHash};

    mod fetch_withdrawal_documents_by_status {

        use super::*;

        #[test]
        fn test_return_list_of_documents() {
            let drive = setup_drive_with_initial_state_structure();

            let transaction = drive.grove.start_transaction();

            let data_contract = get_withdrawals_data_contract_fixture(None);

            setup_system_data_contract(&drive, &data_contract, Some(&transaction));

            let documents = fetch_withdrawal_documents_by_status(
                &drive,
                withdrawals_contract::statuses::QUEUED,
                Some(&transaction),
            )
            .expect("to fetch documents by status");

            assert_eq!(documents.len(), 0);

            let document = get_withdrawal_document_fixture(
                &data_contract,
                json!({
                    "amount": 1000,
                    "coreFeePerByte": 1,
                    "pooling": 0,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::statuses::QUEUED,
                    "transactionIndex": 1,
                }),
            );

            setup_document(&drive, &document, &data_contract, Some(&transaction));

            let document = get_withdrawal_document_fixture(
                &data_contract,
                json!({
                    "amount": 1000,
                    "coreFeePerByte": 1,
                    "pooling": 0,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::statuses::POOLED,
                    "transactionIndex": 2,
                }),
            );

            setup_document(&drive, &document, &data_contract, Some(&transaction));

            let documents = fetch_withdrawal_documents_by_status(
                &drive,
                withdrawals_contract::statuses::QUEUED,
                Some(&transaction),
            )
            .expect("to fetch documents by status");

            assert_eq!(documents.len(), 1);

            let documents = fetch_withdrawal_documents_by_status(
                &drive,
                withdrawals_contract::statuses::POOLED,
                Some(&transaction),
            )
            .expect("to fetch documents by status");

            assert_eq!(documents.len(), 1);
        }
    }

    mod fetch_core_block_transactions {
        use crate::drive::identity::withdrawals::withdrawal_status::fetch_core_block_transactions;

        use super::*;

        #[test]
        fn test_fetches_core_transactions() {
            let mut drive = setup_drive_with_initial_state_structure();

            let mut mock_rpc_client = MockCoreRPCLike::new();

            mock_rpc_client
                .expect_get_block_hash()
                .withf(|height| *height == 1)
                .returning(|_| {
                    Ok(BlockHash::from_hex(
                        "0000000000000000000000000000000000000000000000000000000000000000",
                    )
                    .unwrap())
                });

            mock_rpc_client
                .expect_get_block_hash()
                .withf(|height| *height == 2)
                .returning(|_| {
                    Ok(BlockHash::from_hex(
                        "1111111111111111111111111111111111111111111111111111111111111111",
                    )
                    .unwrap())
                });

            mock_rpc_client
                .expect_get_block_json()
                .withf(|bh| {
                    bh.to_hex()
                        == "0000000000000000000000000000000000000000000000000000000000000000"
                })
                .returning(|_| {
                    Ok(json!({
                        "tx": ["1"]
                    }))
                });

            mock_rpc_client
                .expect_get_block_json()
                .withf(|bh| {
                    bh.to_hex()
                        == "1111111111111111111111111111111111111111111111111111111111111111"
                })
                .returning(|_| {
                    Ok(json!({
                        "tx": ["2"]
                    }))
                });

            drive.core_rpc = Some(Box::new(mock_rpc_client));

            let transactions =
                fetch_core_block_transactions(&drive, 1, 2).expect("to fetch core transactions");

            assert_eq!(transactions.len(), 2);
            assert_eq!(transactions, ["1", "2"]);
        }
    }

    mod update_withdrawal_statuses {
        use super::*;

        #[test]
        fn test_statuses_are_updated() {
            let mut drive = setup_drive_with_initial_state_structure();

            let mut mock_rpc_client = MockCoreRPCLike::new();

            mock_rpc_client
                .expect_get_block_hash()
                .withf(|height| *height == 95)
                .returning(|_| {
                    Ok(BlockHash::from_hex(
                        "0000000000000000000000000000000000000000000000000000000000000000",
                    )
                    .unwrap())
                });

            mock_rpc_client
                .expect_get_block_hash()
                .withf(|height| *height == 96)
                .returning(|_| {
                    Ok(BlockHash::from_hex(
                        "1111111111111111111111111111111111111111111111111111111111111111",
                    )
                    .unwrap())
                });

            mock_rpc_client
                .expect_get_block_json()
                .withf(|bh| {
                    bh.to_hex()
                        == "0000000000000000000000000000000000000000000000000000000000000000"
                })
                .returning(|_| {
                    Ok(json!({
                        "tx": ["0101010101010101010101010101010101010101010101010101010101010101"]
                    }))
                });

            mock_rpc_client
                .expect_get_block_json()
                .withf(|bh| {
                    bh.to_hex()
                        == "1111111111111111111111111111111111111111111111111111111111111111"
                })
                .returning(|_| {
                    Ok(json!({
                        "tx": ["0202020202020202020202020202020202020202020202020202020202020202"]
                    }))
                });

            drive.core_rpc = Some(Box::new(mock_rpc_client));

            let transaction = drive.grove.start_transaction();

            let data_contract = get_withdrawals_data_contract_fixture(None);

            setup_system_data_contract(&drive, &data_contract, Some(&transaction));

            let document_1 = get_withdrawal_document_fixture(
                &data_contract,
                json!({
                    "amount": 1000,
                    "coreFeePerByte": 1,
                    "pooling": 0,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::statuses::BROADCASTED,
                    "transactionIndex": 1,
                    "transactionSignHeight": 93,
                    "transactionId": vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
                }),
            );

            setup_document(&drive, &document_1, &data_contract, Some(&transaction));

            let document_2 = get_withdrawal_document_fixture(
                &data_contract,
                json!({
                    "amount": 1000,
                    "coreFeePerByte": 1,
                    "pooling": 0,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::statuses::BROADCASTED,
                    "transactionIndex": 2,
                    "transactionSignHeight": 10,
                    "transactionId": vec![3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
                }),
            );

            setup_document(&drive, &document_2, &data_contract, Some(&transaction));

            drive
                .update_broadcasted_withdrawal_transaction_statuses(
                    95,
                    96,
                    1,
                    1,
                    1,
                    Some(&transaction),
                )
                .expect("to update withdrawal statuses");

            let documents = fetch_withdrawal_documents_by_status(
                &drive,
                withdrawals_contract::statuses::EXPIRED,
                Some(&transaction),
            )
            .expect("to fetch documents by status");

            assert_eq!(documents.len(), 1);
            assert_eq!(documents.get(0).unwrap().id, document_2.id);

            let documents = fetch_withdrawal_documents_by_status(
                &drive,
                withdrawals_contract::statuses::COMPLETE,
                Some(&transaction),
            )
            .expect("to fetch documents by status");

            assert_eq!(documents.len(), 1);
            assert_eq!(documents.get(0).unwrap().id, document_1.id);
        }
    }
}
