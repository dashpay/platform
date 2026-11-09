//! `getOwnerTransactionsInBlock`: the state transitions one owner has in a block.
//!
//! Drive keeps no record of which transitions a block held, so the answer comes from
//! the block Tenderdash stored. A client trusts the node it asks unless it sets `prove`.

use dapi_grpc::platform::v0::get_owner_transactions_in_block_response::get_owner_transactions_in_block_response_v0::OwnerTransaction;
use dapi_grpc::platform::v0::get_owner_transactions_in_block_response::{
    self, GetOwnerTransactionsInBlockResponseV0,
};
use dapi_grpc::platform::v0::{
    GetOwnerTransactionsInBlockRequest, GetOwnerTransactionsInBlockResponse,
    get_owner_transactions_in_block_request,
};
use dapi_grpc::tonic::{Request, Response};
use dpp::platform_value::Identifier;
use dpp::serialization::PlatformDeserializableUntrusted;
use dpp::state_transition::StateTransition;
use sha2::{Digest, Sha256};

use super::block_proof::{build_block_proof, transaction_proofs};
use super::error_mapping::base64_decode;
use crate::cache::make_cache_key;
use crate::clients::tenderdash_client::{BlockTxResult, MAX_TENDERDASH_HEIGHT};
use crate::error::{DAPIResult, DapiError};
use crate::services::platform_service::PlatformServiceImpl;

impl PlatformServiceImpl {
    /// Read the block at the requested height from Tenderdash and return the state
    /// transitions of the requested owner, each with the result of its execution.
    pub(super) async fn get_owner_transactions_in_block_impl(
        &self,
        request: Request<GetOwnerTransactionsInBlockRequest>,
    ) -> DAPIResult<Response<GetOwnerTransactionsInBlockResponse>> {
        let request = request.into_inner();
        let (height, owner_id, prove) = parse_request(&request)?;

        let key = make_cache_key("get_owner_transactions_in_block", &request);
        if let Some(cached) = self.platform_cache.get(&key) {
            return Ok(Response::new(cached));
        }
        let generation = self.platform_cache.generation();

        let result = self.tenderdash_client.block(height).await?;
        let block_id = result.block_id;
        let block = result
            .block
            .ok_or_else(|| DapiError::NotFound(format!("no block at height {height}")))?;
        let block_txs = block.data.txs;

        let owned = select_owned_transactions(&block_txs, owner_id)?;
        let indexes = owned.iter().map(|tx| tx.index).collect::<Vec<_>>();
        // Most blocks hold nothing of a given owner, so the results are read only when needed
        let results = if owned.is_empty() {
            Vec::new()
        } else {
            self.tenderdash_client
                .block_results(height)
                .await?
                .txs_results
        };
        let mut transactions = if owned.is_empty() {
            Vec::new()
        } else {
            attach_results(owned, &results, block_txs.len())?
        };

        let proof = if prove {
            let commit = self.tenderdash_client.commit(height).await?;
            let quorum_type = self.tenderdash_client.validator_quorum_type().await?;
            let proof = build_block_proof(height, &block_id, commit, quorum_type)?;
            let proofs = transaction_proofs(&proof, &block_txs, &results, &indexes)?;
            for (transaction, transaction_proof) in transactions.iter_mut().zip(proofs) {
                transaction.proof = Some(transaction_proof);
            }
            Some(proof)
        } else {
            None
        };

        let response = GetOwnerTransactionsInBlockResponse {
            version: Some(get_owner_transactions_in_block_response::Version::V0(
                GetOwnerTransactionsInBlockResponseV0 {
                    transactions,
                    proof,
                },
            )),
        };
        self.platform_cache.put_from_generation(
            key,
            &response,
            dapi_grpc::Message::encoded_len(&response),
            generation,
        );

        Ok(Response::new(response))
    }
}

/// Validate the request, returning the height, the owner and whether to prove the answer.
fn parse_request(
    request: &GetOwnerTransactionsInBlockRequest,
) -> DAPIResult<(u64, Identifier, bool)> {
    let Some(get_owner_transactions_in_block_request::Version::V0(v0)) = &request.version else {
        return Err(DapiError::InvalidArgument(
            "missing request version".to_string(),
        ));
    };
    if v0.height == 0 || v0.height > MAX_TENDERDASH_HEIGHT {
        return Err(DapiError::InvalidArgument(format!(
            "height must be between 1 and {MAX_TENDERDASH_HEIGHT}"
        )));
    }
    let owner_id = Identifier::from_bytes(&v0.owner_id)
        .map_err(|_| DapiError::InvalidArgument("owner_id must be 32 bytes".to_string()))?;

    Ok((v0.height, owner_id, v0.prove))
}

/// A state transition of the owner, and its position in the block.
#[derive(Debug, PartialEq)]
struct OwnedTransaction {
    index: usize,
    state_transition: Vec<u8>,
}

/// Pick the transactions of `owner_id` out of a block's base64 encoded transactions.
///
/// Drive executes only transitions it can decode, so one that does not decode here means
/// this DAPI is out of step with the network. That fails the query: the response claims
/// to be every transition of the owner, and the undecodable one might be among them.
fn select_owned_transactions(
    block_txs: &[String],
    owner_id: Identifier,
) -> DAPIResult<Vec<OwnedTransaction>> {
    let mut owned = Vec::new();
    for (index, tx) in block_txs.iter().enumerate() {
        let state_transition = base64_decode(tx).ok_or_else(|| {
            DapiError::Internal(format!("transaction {index} of the block is not base64"))
        })?;
        let decoded = StateTransition::deserialize_from_bytes_untrusted(&state_transition)
            .map_err(|error| {
                DapiError::Internal(format!(
                    "transaction {index} of the block does not decode: {error}"
                ))
            })?;
        if decoded.owner_id() == Some(owner_id) {
            owned.push(OwnedTransaction {
                index,
                state_transition,
            });
        }
    }
    Ok(owned)
}

/// Pair each selected transaction with its execution result.
///
/// Results line up with the block's transactions by position, so a list of another
/// length cannot be trusted to give any transaction its own result.
fn attach_results(
    owned: Vec<OwnedTransaction>,
    results: &[BlockTxResult],
    block_tx_count: usize,
) -> DAPIResult<Vec<OwnerTransaction>> {
    if results.len() != block_tx_count {
        return Err(DapiError::Internal(format!(
            "block has {block_tx_count} transactions but {} results",
            results.len()
        )));
    }
    owned
        .into_iter()
        .map(|transaction| {
            let result = results.get(transaction.index).ok_or_else(|| {
                DapiError::Internal(format!("no result for transaction {}", transaction.index))
            })?;
            let result_data = base64_decode(&result.data).ok_or_else(|| {
                DapiError::Internal(format!(
                    "the result of transaction {} is not base64",
                    transaction.index
                ))
            })?;
            Ok(OwnerTransaction {
                hash: Sha256::digest(&transaction.state_transition).to_vec(),
                state_transition: transaction.state_transition,
                code: result.code,
                result_data,
                gas_used: result.gas_used,
                proof: None,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::tenderdash_client::{ResultBlock, ResultBlockResults};
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use dapi_grpc::platform::v0::get_owner_transactions_in_block_request::GetOwnerTransactionsInBlockRequestV0;
    use dpp::serialization::PlatformSerializable;
    use dpp::shielded::SerializedAction;
    use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
    use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
    use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
    use dpp::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;

    const ALICE: Identifier = Identifier::new([1; 32]);
    const BOB: Identifier = Identifier::new([2; 32]);

    /// A credit transfer from `identity_id` to `recipient_id`, base64 encoded as in a block.
    fn transfer(identity_id: Identifier, recipient_id: Identifier, amount: u64) -> String {
        let transition = StateTransition::IdentityCreditTransfer(
            IdentityCreditTransferTransition::V0(IdentityCreditTransferTransitionV0 {
                identity_id,
                recipient_id,
                amount,
                ..Default::default()
            }),
        );
        STANDARD.encode(transition.serialize_to_bytes().unwrap())
    }

    /// A shielded transfer, which has no owner, base64 encoded as in a block.
    fn shielded_transfer() -> String {
        let transition = StateTransition::ShieldedTransfer(ShieldedTransferTransition::V0(
            ShieldedTransferTransitionV0 {
                actions: vec![SerializedAction {
                    nullifier: [1; 32],
                    rk: [2; 32],
                    cmx: [3; 32],
                    encrypted_note: vec![4; 216],
                    cv_net: [5; 32],
                    spend_auth_sig: [6; 64],
                }],
                value_balance: 1,
                anchor: [7; 32],
                proof: vec![0; 100],
                binding_signature: [0; 64],
            },
        ));
        STANDARD.encode(transition.serialize_to_bytes().unwrap())
    }

    fn request(height: u64, owner_id: Vec<u8>) -> GetOwnerTransactionsInBlockRequest {
        GetOwnerTransactionsInBlockRequest {
            version: Some(get_owner_transactions_in_block_request::Version::V0(
                GetOwnerTransactionsInBlockRequestV0 {
                    height,
                    owner_id,
                    prove: false,
                },
            )),
        }
    }

    #[test]
    fn should_select_only_the_owners_transactions_in_block_order() {
        let txs = [
            transfer(ALICE, BOB, 1),
            transfer(BOB, ALICE, 2),
            transfer(ALICE, BOB, 3),
        ];

        let owned = select_owned_transactions(&txs, ALICE).unwrap();

        assert_eq!(owned.iter().map(|tx| tx.index).collect::<Vec<_>>(), [0, 2]);
        assert_eq!(owned[0].state_transition, STANDARD.decode(&txs[0]).unwrap());
        assert_eq!(owned[1].state_transition, STANDARD.decode(&txs[2]).unwrap());
    }

    #[test]
    fn should_not_select_a_transaction_for_its_recipient() {
        let txs = [transfer(ALICE, BOB, 1)];

        assert!(select_owned_transactions(&txs, BOB).unwrap().is_empty());
    }

    #[test]
    fn should_skip_transactions_without_an_owner() {
        let txs = [shielded_transfer(), transfer(ALICE, BOB, 1)];

        let owned = select_owned_transactions(&txs, ALICE).unwrap();

        assert_eq!(owned.iter().map(|tx| tx.index).collect::<Vec<_>>(), [1]);
    }

    #[test]
    fn should_fail_rather_than_omit_a_transaction_that_does_not_decode() {
        let txs = [transfer(ALICE, BOB, 1), STANDARD.encode([0xff, 0x00])];
        assert!(matches!(
            select_owned_transactions(&txs, ALICE),
            Err(DapiError::Internal(_))
        ));

        let txs = ["not base64!".to_string()];
        assert!(matches!(
            select_owned_transactions(&txs, ALICE),
            Err(DapiError::Internal(_))
        ));
    }

    #[test]
    fn should_give_each_transaction_its_own_result_and_hash() {
        let txs = [transfer(BOB, ALICE, 1), transfer(ALICE, BOB, 2)];
        let owned = select_owned_transactions(&txs, ALICE).unwrap();
        let results = [
            BlockTxResult::default(),
            BlockTxResult {
                code: 40100,
                data: "AQI=".to_string(),
                gas_used: 9,
            },
        ];

        let transactions = attach_results(owned, &results, txs.len()).unwrap();

        let bytes = STANDARD.decode(&txs[1]).unwrap();
        assert_eq!(transactions.len(), 1);
        assert_eq!(transactions[0].code, 40100);
        assert_eq!(transactions[0].result_data, [1, 2]);
        assert_eq!(transactions[0].gas_used, 9);
        assert!(transactions[0].proof.is_none());
        assert_eq!(transactions[0].hash, Sha256::digest(&bytes).to_vec());
        assert_eq!(transactions[0].state_transition, bytes);
    }

    #[test]
    fn should_fail_when_results_do_not_line_up_with_the_block() {
        let txs = [transfer(ALICE, BOB, 1), transfer(ALICE, BOB, 2)];
        let owned = select_owned_transactions(&txs, ALICE).unwrap();

        assert!(matches!(
            attach_results(owned, &[BlockTxResult::default()], txs.len()),
            Err(DapiError::Internal(_))
        ));
    }

    #[test]
    fn should_accept_a_valid_request() {
        let (height, owner_id, prove) = parse_request(&request(7, vec![1; 32])).unwrap();

        assert_eq!(height, 7);
        assert_eq!(owner_id, ALICE);
        assert!(!prove);
    }

    #[test]
    fn should_read_the_prove_flag() {
        let mut proved = request(7, vec![1; 32]);
        let Some(get_owner_transactions_in_block_request::Version::V0(v0)) = &mut proved.version
        else {
            unreachable!();
        };
        v0.prove = true;

        assert!(parse_request(&proved).unwrap().2);
    }

    #[test]
    fn should_reject_a_height_tenderdash_cannot_hold() {
        for height in [0, MAX_TENDERDASH_HEIGHT + 1] {
            assert!(matches!(
                parse_request(&request(height, vec![1; 32])),
                Err(DapiError::InvalidArgument(_))
            ));
        }
    }

    #[test]
    fn should_reject_an_owner_id_of_the_wrong_length() {
        assert!(matches!(
            parse_request(&request(1, vec![1; 31])),
            Err(DapiError::InvalidArgument(_))
        ));
    }

    #[test]
    fn should_reject_a_request_without_a_version() {
        let request = GetOwnerTransactionsInBlockRequest { version: None };

        assert!(matches!(
            parse_request(&request),
            Err(DapiError::InvalidArgument(_))
        ));
    }

    #[test]
    fn should_read_tenderdash_block_json() {
        let block: ResultBlock = serde_json::from_value(serde_json::json!({
            "block_id": { "hash": "AA" },
            "block": { "header": { "height": "5" }, "data": { "txs": ["AQ==", "Ag=="] } },
        }))
        .unwrap();

        assert_eq!(block.block.unwrap().data.txs, ["AQ==", "Ag=="]);
    }

    #[test]
    fn should_read_an_empty_block_and_a_missing_block_from_tenderdash_json() {
        let empty: ResultBlock =
            serde_json::from_value(serde_json::json!({ "block": { "data": { "txs": null } } }))
                .unwrap();
        assert!(empty.block.unwrap().data.txs.is_empty());

        let missing: ResultBlock =
            serde_json::from_value(serde_json::json!({ "block_id": {}, "block": null })).unwrap();
        assert!(missing.block.is_none());
    }

    #[test]
    fn should_read_tenderdash_block_results_json() {
        let results: ResultBlockResults = serde_json::from_value(serde_json::json!({
            "height": "5",
            "txs_results": [{ "gas_used": "9" }, { "code": 40100, "gas_used": "1" }],
        }))
        .unwrap();
        assert_eq!(
            results
                .txs_results
                .iter()
                .map(|r| r.code)
                .collect::<Vec<_>>(),
            [0, 40100]
        );

        let empty: ResultBlockResults =
            serde_json::from_value(serde_json::json!({ "txs_results": null })).unwrap();
        assert!(empty.txs_results.is_empty());
    }
}
