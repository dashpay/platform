use dpp::block::block_info::BlockInfo;
use dpp::dashcore::transaction::special_transaction::asset_unlock::qualified_asset_unlock::build_asset_unlock_tx;
use dpp::dashcore::Transaction;
use dpp::dashcore::{
    blockdata::transaction::special_transaction::asset_unlock::request_info::AssetUnlockRequestInfo,
    hashes::Hash, QuorumHash,
};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::{Document, DocumentV0Setters};
use dpp::version::PlatformVersion;

use drive::dpp::system_data_contracts::withdrawals_contract;
use drive::dpp::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;

use dpp::withdrawal::WithdrawalTransactionIndex;
use drive::query::TransactionArg;
use drive::util::batch::DriveOperation;

use crate::platform_types::withdrawal::unsigned_withdrawal_txs::v0::UnsignedWithdrawalTxs;
use crate::rpc::core::CoreHeight;
use crate::{
    error::{execution::ExecutionError, Error},
    platform_types::platform::Platform,
    rpc::core::CoreRPCLike,
};
use dpp::errors::ProtocolError;

use drive::config::DEFAULT_QUERY_LIMIT;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Prepares a list of an unsigned withdrawal transaction bytes
    pub(super) fn dequeue_and_build_unsigned_withdrawal_transactions_v0(
        &self,
        validator_set_quorum_hash: [u8; 32],
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<UnsignedWithdrawalTxs, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];

        // Get withdrawal_transactions_per_block_limit (normally 4) latest withdrawal transactions from the queue
        let untied_withdrawal_transactions = self.drive.dequeue_untied_withdrawal_transactions(
            platform_version
                .system_limits
                .withdrawal_transactions_per_block_limit,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;

        if untied_withdrawal_transactions.is_empty() {
            return Ok(UnsignedWithdrawalTxs::default());
        }

        let transaction_indices = untied_withdrawal_transactions
            .iter()
            .map(|(transaction_id, _)| *transaction_id)
            .collect::<Vec<_>>();

        let documents = self.fetch_and_modify_withdrawal_documents_to_broadcasted_by_indices(
            &transaction_indices,
            block_info,
            transaction,
            platform_version,
        )?;

        tracing::debug!(
            "Deque {} unsigned withdrawal transactions for signing with indices from {:?} to {:?}",
            documents.len(),
            transaction_indices.first(),
            transaction_indices.last()
        );

        let withdrawals_contract = self
            .drive
            .cache
            .system_data_contracts
            .load_withdrawals(platform_version)?;

        self.drive.add_update_multiple_documents_operations(
            &documents,
            &withdrawals_contract,
            withdrawals_contract
                .document_type_for_name(withdrawal::NAME)
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "could not get document type",
                    ))
                })?,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        // Appending request_height and quorum_hash to withdrawal transaction
        let unsigned_withdrawal_transactions = untied_withdrawal_transactions
            .into_iter()
            .map(|(_, untied_transaction_bytes)| {
                build_unsigned_transaction(
                    untied_transaction_bytes,
                    validator_set_quorum_hash,
                    block_info.core_height,
                )
            })
            .collect::<Result<_, _>>()?;

        self.drive.apply_drive_operations(
            drive_operations,
            true,
            block_info,
            transaction,
            platform_version,
            None,
        )?;

        Ok(UnsignedWithdrawalTxs::from_vec(
            unsigned_withdrawal_transactions,
        ))
    }

    fn fetch_and_modify_withdrawal_documents_to_broadcasted_by_indices(
        &self,
        transaction_indices: &[WithdrawalTransactionIndex],
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, Error> {
        let documents = self
            .drive
            .find_withdrawal_documents_by_status_and_transaction_indices(
                withdrawals_contract::WithdrawalStatus::POOLED,
                transaction_indices,
                DEFAULT_QUERY_LIMIT,
                transaction,
                platform_version,
            )?;

        documents
            .into_iter()
            .map(|mut document| {
                document.set_i64(
                    withdrawal::properties::STATUS,
                    withdrawals_contract::WithdrawalStatus::BROADCASTED as i64,
                );

                document.set_u64(
                    withdrawal::properties::TRANSACTION_SIGN_HEIGHT,
                    block_info.core_height as u64,
                );

                document.set_updated_at(Some(block_info.time_ms));

                document.increment_revision().map_err(|_| {
                    Error::Execution(ExecutionError::Overflow(
                        "Overflow when adding to document revision for withdrawals",
                    ))
                })?;

                Ok(document)
            })
            .collect()
    }
}

fn build_unsigned_transaction(
    untied_transaction_bytes: Vec<u8>,
    mut validator_set_quorum_hash: [u8; 32],
    core_chain_locked_height: CoreHeight,
) -> Result<Transaction, Error> {
    // Core expects it reversed
    validator_set_quorum_hash.reverse();

    let request_info = AssetUnlockRequestInfo {
        request_height: core_chain_locked_height,
        quorum_hash: QuorumHash::from_byte_array(validator_set_quorum_hash),
    };

    let mut unsigned_transaction_bytes = vec![];

    request_info
        .consensus_append_to_base_encode(
            untied_transaction_bytes.clone(),
            &mut unsigned_transaction_bytes,
        )
        .map_err(|_| {
            Error::Execution(ExecutionError::CorruptedCodeExecution(
                "could not add additional request info to asset unlock transaction",
            ))
        })?;

    build_asset_unlock_tx(&unsigned_transaction_bytes)
        .map_err(|error| Error::Protocol(ProtocolError::DashCoreError(error)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::epoch::Epoch;
    use dpp::dashcore::consensus;
    use dpp::dashcore::transaction::special_transaction::asset_unlock::unqualified_asset_unlock::{
        AssetUnlockBasePayload, AssetUnlockBaseTransactionInfo,
    };
    use dpp::dashcore::transaction::special_transaction::TransactionPayload::AssetUnlockPayloadType;
    use dpp::dashcore::{PubkeyHash, ScriptBuf, TxOut, Txid};
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use drive::util::test_helpers::setup::setup_system_data_contract;
    use std::str::FromStr;

    /// The untied transaction of the DIP-0027 worked examples at `index`: a version 2 asset
    /// unlock paying 1 Dash to the P2PKH script of `[0x11; 20]` with a 70,000 duff fee. The
    /// pooling step only builds fees that are multiples of 190 duffs, so it is queued as is.
    fn dip_0027_untied_transaction(index: WithdrawalTransactionIndex) -> Vec<u8> {
        consensus::serialize(&AssetUnlockBaseTransactionInfo {
            version: 1,
            lock_time: 0,
            output: vec![TxOut {
                value: 100_000_000,
                script_pubkey: ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array([0x11; 20])),
            }],
            base_payload: AssetUnlockBasePayload {
                version: 2,
                index,
                fee: 70_000,
            },
        })
    }

    /// Signs what the queue holds at `core_height` with the validator set `quorum_hash`.
    fn dequeue(
        platform: &Platform<MockCoreRPCLike>,
        quorum_hash: [u8; 32],
        core_height: CoreHeight,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Vec<Transaction> {
        let block_info = BlockInfo {
            time_ms: 1,
            height: 1,
            core_height,
            epoch: Epoch::default(),
        };
        platform
            .dequeue_and_build_unsigned_withdrawal_transactions(
                quorum_hash,
                &block_info,
                transaction,
                platform_version,
            )
            .expect("expected to sign the queued withdrawals")
            .into_iter()
            .collect()
    }

    /// Every instance Platform signs of a version 2 withdrawal must have Core's stable txid:
    /// an InstantSend lock on the withdrawal pins that txid, so an instance with another one
    /// could never be mined. The DIP-0027 txids are pinned through the whole signing path,
    /// both when the withdrawal is first signed and when the expiry step queues the same bytes
    /// again and it is signed at a later height by another quorum. This guards the transforms
    /// from untied bytes to the signed transaction, in Platform and in rust-dashcore, against
    /// any change that would move the txid.
    #[test]
    fn should_give_a_version_2_withdrawal_cores_stable_txid_through_the_signing_pipeline() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let transaction = platform.drive.grove.start_transaction();
        let data_contract =
            load_system_data_contract(SystemDataContract::Withdrawals, platform_version)
                .expect("to load system data contract");
        setup_system_data_contract(&platform.drive, &data_contract, Some(&transaction));

        let dip_0027_txids = [
            (
                101,
                "3c4db73c8356407a5d7c78df5045bd280f2dc4fd644b06c4bfbdead3d5ae41cf",
            ),
            (
                123_456_789,
                "a67e1107ae6e04b813bc8e81348266f5206d1ca93d305dc4323940e18cdbaf34",
            ),
        ]
        .map(|(index, txid)| {
            (
                index,
                Txid::from_str(txid).expect("expected a txid as Core prints it"),
            )
        });
        let indices = dip_0027_txids.map(|(index, _)| index).to_vec();

        let mut drive_operations = vec![];
        platform
            .drive
            .add_enqueue_untied_withdrawal_transaction_operations(
                indices
                    .iter()
                    .map(|index| (*index, dip_0027_untied_transaction(*index)))
                    .collect(),
                0,
                &mut drive_operations,
                platform_version,
            )
            .expect("expected to queue the withdrawals");
        platform
            .drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("expected to queue the withdrawals");

        let first = dequeue(
            &platform,
            [1; 32],
            500,
            Some(&transaction),
            platform_version,
        );

        let mut drive_operations = vec![];
        platform
            .drive
            .move_broadcasted_withdrawal_transactions_back_to_queue_operations(
                indices,
                &mut drive_operations,
                platform_version,
            )
            .expect("expected to queue the withdrawals again");
        platform
            .drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("expected to queue the withdrawals again");

        let re_signed = dequeue(
            &platform,
            [2; 32],
            549,
            Some(&transaction),
            platform_version,
        );

        assert_eq!(first.len(), dip_0027_txids.len());
        assert_eq!(re_signed.len(), dip_0027_txids.len());
        for (((index, txid), first), re_signed) in dip_0027_txids.iter().zip(&first).zip(&re_signed)
        {
            for (instance, request_height) in [(first, 500), (re_signed, 549)] {
                let Some(AssetUnlockPayloadType(payload)) = &instance.special_transaction_payload
                else {
                    panic!("expected an asset unlock");
                };
                assert_eq!(payload.base.index, *index);
                assert_eq!(payload.request_info.request_height, request_height);
                assert_eq!(&instance.txid(), txid, "index {index}");
            }
            assert_ne!(
                first.instance_hash(),
                re_signed.instance_hash(),
                "index {index}: a re-signed instance is relayed under a new instance hash"
            );
        }
    }

    /// When the withdrawal queue is empty, the function must return an empty
    /// `UnsignedWithdrawalTxs` without performing any further work. This
    /// exercises the `untied_withdrawal_transactions.is_empty()` early-return
    /// branch.
    #[test]
    fn v0_empty_queue_returns_default() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let transaction = platform.drive.grove.start_transaction();

        let block_info = BlockInfo {
            time_ms: 1,
            height: 1,
            core_height: 96,
            epoch: Epoch::default(),
        };

        let result = platform
            .dequeue_and_build_unsigned_withdrawal_transactions_v0(
                [0u8; 32],
                &block_info,
                Some(&transaction),
                platform_version,
            )
            .expect("empty queue must be Ok");

        // default UnsignedWithdrawalTxs is empty.
        assert!(
            result.is_empty(),
            "empty withdrawal queue must yield an empty UnsignedWithdrawalTxs"
        );
    }

    /// `build_unsigned_transaction` reverses the quorum hash and must succeed
    /// for a well-formed untied asset-unlock payload. This covers the happy
    /// path of the helper, which is otherwise only reachable from the full
    /// dequeue flow.
    #[test]
    fn build_unsigned_transaction_invalid_bytes_yields_error() {
        // All-zero bytes are not a valid untied asset-unlock transaction; the
        // `build_asset_unlock_tx` call should fail with a protocol error.
        let result = build_unsigned_transaction(vec![0u8; 8], [0u8; 32], 100);

        let err = result.expect_err("invalid bytes must produce an error");
        // Should either be Protocol (DashCoreError) or Execution (CorruptedCodeExecution)
        // from the consensus append path.
        let msg = err.to_string();
        assert!(
            !msg.is_empty(),
            "error must have a non-empty description: {:?}",
            err
        );
    }
}
