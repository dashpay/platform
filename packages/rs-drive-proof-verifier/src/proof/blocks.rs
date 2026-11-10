//! The answers of `getBlocks` and `getOwnerTransactionsInBlock`, held to what was asked
//! and, when proved, to the Tenderdash block proofs they carry.
//!
//! These answers are about blocks, not about Platform's state, so they come with a
//! [`TenderdashBlockProof`](dapi_grpc::platform::v0::TenderdashBlockProof) per block and
//! neither a GroveDB proof nor response metadata. A verified answer says nothing about how
//! recent the blocks are.

use crate::block_proof::{
    check_block_contents, check_block_fields, result_leaf, transaction_hash, verify_block_proof,
    verify_transaction_proof, BlockProofError,
};
use crate::types::blocks::{Block, BlockTransaction, Blocks, OwnerTransactionsInBlock};
use crate::{ContextProvider, Error};
use dapi_grpc::platform::v0::get_blocks_response::get_blocks_response_v0::{
    Block as BlockProto, Transaction as TransactionProto,
};
use dapi_grpc::platform::v0::get_owner_transactions_in_block_response::get_owner_transactions_in_block_response_v0::OwnerTransaction as OwnerTransactionProto;
use dapi_grpc::platform::v0::tenderdash_block_proof::Header;
use dapi_grpc::platform::v0::{
    get_blocks_request, get_blocks_response, get_owner_transactions_in_block_request,
    get_owner_transactions_in_block_response, GetBlocksRequest, GetBlocksResponse,
    GetOwnerTransactionsInBlockRequest, GetOwnerTransactionsInBlockResponse,
};
use dpp::platform_value::Identifier;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;

/// The protocol version the block of a verified `header` executed under, which decides
/// what decodes in that block. A block of a version newer than any this build knows is
/// read with the newest rules there are.
///
/// The header is signed, so a version this build cannot read is what every honest node
/// answers with: no node is to blame for it.
fn block_platform_version(header: &Header) -> Result<&'static PlatformVersion, Error> {
    let latest = PlatformVersion::latest();
    match u32::try_from(header.app_version) {
        Ok(version) if version <= latest.protocol_version => {
            PlatformVersion::get(version).map_err(|error| Error::UnusableVerifiedResult {
                reason: format!(
                    "block {} executed under protocol version {version}, which this build \
                     cannot read: {error}",
                    header.height
                ),
            })
        }
        _ => Ok(latest),
    }
}

/// The error for a transaction a verified block holds that does not decode under the
/// block's protocol version. Every honest node answers with the same bytes, so no node is
/// to blame and asking another cannot help.
fn committed_but_unreadable(
    height: u64,
    position: usize,
    error: ProtocolError,
    platform_version: &PlatformVersion,
) -> Error {
    Error::UnusableVerifiedResult {
        reason: format!(
            "transaction {position} of block {height} is in the block but does not decode \
             under protocol version {}: {error}",
            platform_version.protocol_version
        ),
    }
}

/// The error for a transaction of an unproved answer that does not decode.
fn unreadable(height: u64, position: usize, error: ProtocolError) -> Error {
    Error::ProtocolError {
        error: format!("transaction {position} of block {height} does not decode: {error}"),
    }
}

/// A transaction as an answer carries it.
struct AnsweredTransaction<'a> {
    state_transition: &'a [u8],
    hash: &'a [u8],
    code: u32,
    result_data: &'a [u8],
    gas_used: i64,
}

impl<'a> From<&'a OwnerTransactionProto> for AnsweredTransaction<'a> {
    fn from(transaction: &'a OwnerTransactionProto) -> Self {
        Self {
            state_transition: &transaction.state_transition,
            hash: &transaction.hash,
            code: transaction.code,
            result_data: &transaction.result_data,
            gas_used: transaction.gas_used,
        }
    }
}

impl<'a> From<&'a TransactionProto> for AnsweredTransaction<'a> {
    fn from(transaction: &'a TransactionProto) -> Self {
        Self {
            state_transition: &transaction.state_transition,
            hash: &transaction.hash,
            code: transaction.code,
            result_data: &transaction.result_data,
            gas_used: transaction.gas_used,
        }
    }
}

impl AnsweredTransaction<'_> {
    /// The hash Tenderdash knows the transaction by, which must be the one it came with.
    /// `position` is the transaction's place in the answer.
    fn hash(&self, height: u64, position: usize) -> Result<[u8; 32], BlockProofError> {
        let hash = transaction_hash(self.state_transition);
        if hash.as_slice() != self.hash {
            return Err(BlockProofError::TransactionHashMismatch { height, position });
        }
        Ok(hash)
    }

    fn result_leaf(&self) -> Vec<u8> {
        result_leaf(self.code, self.result_data, self.gas_used)
    }

    /// Decode the transaction as Drive decoded it, under `platform_version`. `hash` is
    /// what [`Self::hash`] returned. Only what the result keeps is copied.
    fn decode(
        &self,
        hash: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> Result<BlockTransaction, ProtocolError> {
        let state_transition = StateTransition::deserialize_from_bytes_untrusted_in_version(
            self.state_transition,
            platform_version,
        )?;

        Ok(BlockTransaction {
            state_transition,
            hash,
            code: self.code,
            result_data: self.result_data.to_vec(),
            gas_used: self.gas_used,
        })
    }

    /// Decode the transaction of an unproved answer, under a protocol version the caller
    /// chose. Its hash is computed here: the one it came with is the node's word.
    fn decode_unproved(
        &self,
        height: u64,
        position: usize,
        platform_version: &PlatformVersion,
    ) -> Result<BlockTransaction, Error> {
        self.decode(transaction_hash(self.state_transition), platform_version)
            .map_err(|error| unreadable(height, position, error))
    }
}

fn fixed<const N: usize>(height: u64, field: &'static str, bytes: &[u8]) -> Result<[u8; N], Error> {
    bytes.try_into().map_err(|_| Error::ResponseDecodeError {
        error: format!(
            "the {field} of block {height} has {} bytes, not {N}",
            bytes.len()
        ),
    })
}

fn owner_request(request: &GetOwnerTransactionsInBlockRequest) -> Result<(u64, Identifier), Error> {
    let Some(get_owner_transactions_in_block_request::Version::V0(v0)) = &request.version else {
        return Err(Error::EmptyVersion);
    };
    let owner_id = Identifier::from_bytes(&v0.owner_id).map_err(|error| Error::RequestError {
        error: format!("owner_id: {error}"),
    })?;
    Ok((v0.height, owner_id))
}

impl OwnerTransactionsInBlock {
    /// Verify the answer to a `request` that asked to prove.
    ///
    /// The block proof must be of the requested height and hold
    /// ([`verify_block_proof`]). Every transaction must come with Merkle paths that lead
    /// from it and from its result to that block's header
    /// ([`verify_transaction_proof`]), must decode under the block's protocol version, and
    /// must report the requested owner as its own. The transactions must be in block
    /// order, none twice.
    ///
    /// A transaction that is proved to be in the block and does not decode is not an
    /// invalid proof: it is [`Error::UnusableVerifiedResult`], as every node's answer
    /// would be.
    ///
    /// See [`OwnerTransactionsInBlock`] for what the proof leaves open.
    pub fn from_proved_response(
        request: &GetOwnerTransactionsInBlockRequest,
        response: &GetOwnerTransactionsInBlockResponse,
        provider: &dyn ContextProvider,
    ) -> Result<Self, Error> {
        let (height, owner_id) = owner_request(request)?;
        let Some(get_owner_transactions_in_block_response::Version::V0(response)) =
            &response.version
        else {
            return Err(Error::EmptyVersion);
        };
        let proof = response.proof.as_ref().ok_or(Error::NoProofInResult)?;

        let header = verify_block_proof(proof, provider)?;
        if header.height != height {
            return Err(BlockProofError::WrongBlock {
                requested: height,
                proved: header.height,
            }
            .into());
        }
        let platform_version = block_platform_version(header)?;

        let mut previous: Option<(u32, u32)> = None;
        let transactions = response
            .transactions
            .iter()
            .enumerate()
            .map(|(position, transaction)| {
                let proof = transaction
                    .proof
                    .as_ref()
                    .ok_or(BlockProofError::MissingTransactionProof { height, position })?;
                match previous {
                    Some((_, total)) if total != proof.total => {
                        return Err(BlockProofError::InconsistentTotal { height }.into());
                    }
                    Some((index, _)) if index >= proof.index => {
                        return Err(BlockProofError::NotInBlockOrder { height }.into());
                    }
                    _ => previous = Some((proof.index, proof.total)),
                }

                let transaction = AnsweredTransaction::from(transaction);
                let hash = transaction.hash(height, position)?;
                verify_transaction_proof(header, &hash, &transaction.result_leaf(), proof)?;

                let transaction = transaction
                    .decode(hash, platform_version)
                    .map_err(|error| {
                        committed_but_unreadable(height, position, error, platform_version)
                    })?;
                if transaction.state_transition.owner_id() != Some(owner_id) {
                    return Err(BlockProofError::NotTheOwners { height, position }.into());
                }
                Ok(transaction)
            })
            .collect::<Result<Vec<_>, Error>>()?;

        Ok(Self {
            height,
            transactions,
        })
    }

    /// Read the answer to a `request` that did not ask to prove. Nothing here is verified:
    /// the answer is the node's word. The transactions are decoded under
    /// `platform_version`.
    pub fn from_unproved_response(
        request: &GetOwnerTransactionsInBlockRequest,
        response: &GetOwnerTransactionsInBlockResponse,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let (height, _) = owner_request(request)?;
        let Some(get_owner_transactions_in_block_response::Version::V0(response)) =
            &response.version
        else {
            return Err(Error::EmptyVersion);
        };

        let transactions = response
            .transactions
            .iter()
            .enumerate()
            .map(|(position, transaction)| {
                AnsweredTransaction::from(transaction).decode_unproved(
                    height,
                    position,
                    platform_version,
                )
            })
            .collect::<Result<Vec<_>, Error>>()?;

        Ok(Self {
            height,
            transactions,
        })
    }
}

/// The start height and count of a request, and the blocks of its answer.
fn blocks_of_response<'a>(
    request: &GetBlocksRequest,
    response: &'a GetBlocksResponse,
) -> Result<(u64, u32, &'a [BlockProto]), Error> {
    let Some(get_blocks_request::Version::V0(request)) = &request.version else {
        return Err(Error::EmptyVersion);
    };
    let Some(get_blocks_response::Version::V0(response)) = &response.version else {
        return Err(Error::EmptyVersion);
    };
    Ok((request.start_height, request.count, &response.blocks))
}

/// Whether an answer holds more blocks than its request asked for.
fn more_than_asked(count: u32, blocks: &[BlockProto]) -> bool {
    u32::try_from(blocks.len()).map_or(true, |returned| returned > count)
}

/// The height the block at `position` of an answer from `start_height` must have.
fn expected_height(start_height: u64, position: usize) -> Result<u64, Error> {
    u64::try_from(position)
        .ok()
        .and_then(|position| start_height.checked_add(position))
        .ok_or_else(|| Error::RequestError {
            error: format!("no block {position} after height {start_height}"),
        })
}

fn wrong_block(requested: u64, proved: u64) -> Error {
    BlockProofError::WrongBlock { requested, proved }.into()
}

/// Verify one block of a proved answer, which must be the block at `height`.
fn proved_block(
    height: u64,
    block: &BlockProto,
    provider: &dyn ContextProvider,
) -> Result<Block, Error> {
    let proof = block.proof.as_ref().ok_or(Error::NoProofInResult)?;
    let header = verify_block_proof(proof, provider)?;
    if header.height != height {
        return Err(wrong_block(height, header.height));
    }

    // What the answer says about the block next to the proof must be what the proof says
    check_block_fields(proof, block)?;

    // A block holds all its transactions, so the two roots are recomputed from them
    let transactions = block
        .transactions
        .iter()
        .map(AnsweredTransaction::from)
        .collect::<Vec<_>>();
    let hashes = transactions
        .iter()
        .enumerate()
        .map(|(position, transaction)| transaction.hash(height, position))
        .collect::<Result<Vec<_>, _>>()?;
    let result_leaves = transactions
        .iter()
        .map(AnsweredTransaction::result_leaf)
        .collect::<Vec<_>>();
    check_block_contents(header, &hashes, &result_leaves)?;

    let platform_version = block_platform_version(header)?;
    let transactions = transactions
        .iter()
        .zip(hashes)
        .enumerate()
        .map(|(position, (transaction, hash))| {
            transaction.decode(hash, platform_version).map_err(|error| {
                committed_but_unreadable(height, position, error, platform_version)
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;

    block_summary(height, block, transactions)
}

/// The block of an answer, with `transactions` as its already decoded transactions.
fn block_summary(
    height: u64,
    block: &BlockProto,
    transactions: Vec<BlockTransaction>,
) -> Result<Block, Error> {
    Ok(Block {
        height,
        hash: fixed(height, "hash", &block.hash)?,
        time_ms: block.time_ms,
        core_chain_locked_height: block.core_chain_locked_height,
        app_hash: fixed(height, "app hash", &block.app_hash)?,
        proposer_pro_tx_hash: fixed(height, "proposer", &block.proposer_pro_tx_hash)?,
        transactions,
    })
}

impl Blocks {
    /// Verify the answer to a `request` that asked to prove.
    ///
    /// The blocks must be consecutive from the requested start height and no more than
    /// were asked for. Each must come with a block proof that holds
    /// ([`verify_block_proof`]), repeat the fields of its signed header
    /// ([`check_block_fields`]), and hold
    /// exactly the transactions and results that header commits to
    /// ([`check_block_contents`]). The transactions must decode under their block's
    /// protocol version.
    ///
    /// A transaction a proved block holds that does not decode is not an invalid proof:
    /// it is [`Error::UnusableVerifiedResult`], as every node's answer would be.
    ///
    /// See [`Blocks`] for what the proofs leave open.
    pub fn from_proved_response(
        request: &GetBlocksRequest,
        response: &GetBlocksResponse,
        provider: &dyn ContextProvider,
    ) -> Result<Self, Error> {
        let (start_height, count, blocks) = blocks_of_response(request, response)?;
        if more_than_asked(count, blocks) {
            return Err(BlockProofError::TooManyBlocks {
                requested: count,
                returned: blocks.len(),
            }
            .into());
        }
        blocks
            .iter()
            .enumerate()
            .map(|(position, block)| {
                proved_block(expected_height(start_height, position)?, block, provider)
            })
            .collect::<Result<Vec<_>, Error>>()
            .map(Self)
    }

    /// Read the answer to a `request` that did not ask to prove. Nothing here is verified:
    /// the answer is the node's word, held only to the heights that were asked for. The
    /// transactions are decoded under `platform_version`.
    pub fn from_unproved_response(
        request: &GetBlocksRequest,
        response: &GetBlocksResponse,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let (start_height, count, blocks) = blocks_of_response(request, response)?;
        if more_than_asked(count, blocks) {
            return Err(Error::ResponseDecodeError {
                error: format!("asked for {count} blocks but {} came back", blocks.len()),
            });
        }
        blocks
            .iter()
            .enumerate()
            .map(|(position, block)| {
                let height = expected_height(start_height, position)?;
                if block.height != height {
                    return Err(Error::ResponseDecodeError {
                        error: format!(
                            "block {position} of the answer is at height {}, not {height}",
                            block.height
                        ),
                    });
                }
                let transactions = block
                    .transactions
                    .iter()
                    .enumerate()
                    .map(|(position, transaction)| {
                        AnsweredTransaction::from(transaction).decode_unproved(
                            height,
                            position,
                            platform_version,
                        )
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                block_summary(height, block, transactions)
            })
            .collect::<Result<Vec<_>, Error>>()
            .map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_proof::fixtures::{
        self, block_answer, block_answer_of, header_of, owner_answer_of, TestQuorum,
        TestTransaction, CORE_HEIGHT, QUORUM_HASH,
    };
    use dapi_grpc::platform::v0::get_blocks_request::GetBlocksRequestV0;
    use dapi_grpc::platform::v0::get_blocks_response::GetBlocksResponseV0;
    use dapi_grpc::platform::v0::get_owner_transactions_in_block_request::GetOwnerTransactionsInBlockRequestV0;
    use dapi_grpc::platform::v0::get_owner_transactions_in_block_response::GetOwnerTransactionsInBlockResponseV0;
    use dpp::platform_value::Value;
    use dpp::serialization::PlatformSerializable;
    use dpp::shielded::SerializedAction;
    use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
    use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
    use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
    use dpp::state_transition::batch_transition::document_create_transition::v0::DocumentCreateTransitionV0;
    use dpp::state_transition::batch_transition::document_create_transition::DocumentCreateTransition;
    use dpp::state_transition::batch_transition::{BatchTransition, BatchTransitionV1};
    use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
    use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
    use dpp::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;
    use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
    use std::collections::BTreeMap;

    const ALICE: Identifier = Identifier::new([1; 32]);
    const BOB: Identifier = Identifier::new([2; 32]);
    const HEIGHT: u64 = 7;

    fn transfer(identity_id: Identifier, amount: u64) -> StateTransition {
        StateTransition::IdentityCreditTransfer(IdentityCreditTransferTransition::V0(
            IdentityCreditTransferTransitionV0 {
                identity_id,
                recipient_id: Identifier::new([3; 32]),
                amount,
                ..Default::default()
            },
        ))
    }

    /// A shielded transfer, which has no owner.
    fn shielded_transfer() -> StateTransition {
        StateTransition::ShieldedTransfer(ShieldedTransferTransition::V0(
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
        ))
    }

    /// The transitions of the test block: Alice's are at positions 1 and 4.
    fn transitions() -> Vec<StateTransition> {
        vec![
            transfer(BOB, 10),
            transfer(ALICE, 11),
            shielded_transfer(),
            transfer(BOB, 13),
            transfer(ALICE, 14),
        ]
    }

    /// The test block's transactions, each with a result of its own.
    fn block_transactions() -> Vec<TestTransaction> {
        transitions()
            .iter()
            .zip(0u32..)
            .map(|(transition, index)| {
                (
                    transition.serialize_to_bytes().unwrap(),
                    index * 100,
                    vec![index as u8; index as usize],
                    i64::from(index) + 1,
                )
            })
            .collect()
    }

    fn block_transaction(index: usize) -> BlockTransaction {
        let (bytes, code, result_data, gas_used) = block_transactions().remove(index);
        BlockTransaction {
            state_transition: transitions().remove(index),
            hash: transaction_hash(&bytes),
            code,
            result_data,
            gas_used,
        }
    }

    fn owner_request(height: u64, owner_id: Identifier) -> GetOwnerTransactionsInBlockRequest {
        GetOwnerTransactionsInBlockRequest {
            version: Some(get_owner_transactions_in_block_request::Version::V0(
                GetOwnerTransactionsInBlockRequestV0 {
                    height,
                    owner_id: owner_id.to_vec(),
                    prove: true,
                },
            )),
        }
    }

    /// What an honest node answers when asked to prove the transactions at `indexes` of
    /// the test block.
    fn owner_answer(indexes: &[usize]) -> GetOwnerTransactionsInBlockResponseV0 {
        fixtures::owner_answer(HEIGHT, &block_transactions(), indexes)
    }

    fn verify_owner(
        owner_id: Identifier,
        answer: GetOwnerTransactionsInBlockResponseV0,
    ) -> Result<OwnerTransactionsInBlock, Error> {
        OwnerTransactionsInBlock::from_proved_response(
            &owner_request(HEIGHT, owner_id),
            &GetOwnerTransactionsInBlockResponse {
                version: Some(get_owner_transactions_in_block_response::Version::V0(
                    answer,
                )),
            },
            &TestQuorum::new(),
        )
    }

    fn block_proof_error<T: std::fmt::Debug>(result: Result<T, Error>) -> BlockProofError {
        match result {
            Err(Error::InvalidBlockProof(error)) => error,
            other => panic!("expected a block proof error, got {other:?}"),
        }
    }

    #[test]
    fn should_return_the_transactions_of_the_owner_in_a_proved_block() {
        let verified = verify_owner(ALICE, owner_answer(&[1, 4])).unwrap();

        assert_eq!(
            verified,
            OwnerTransactionsInBlock {
                height: HEIGHT,
                transactions: vec![block_transaction(1), block_transaction(4)],
            }
        );
    }

    #[test]
    fn should_accept_a_proved_block_that_holds_nothing_of_the_owner() {
        let verified = verify_owner(Identifier::new([9; 32]), owner_answer(&[])).unwrap();

        assert!(verified.transactions.is_empty());
    }

    #[test]
    fn should_refuse_an_owner_answer_without_a_block_proof() {
        let mut answer = owner_answer(&[1]);
        answer.proof = None;

        assert!(matches!(
            verify_owner(ALICE, answer),
            Err(Error::NoProofInResult)
        ));
    }

    #[test]
    fn should_refuse_an_owner_answer_whose_block_the_quorum_did_not_sign() {
        let mut answer = owner_answer(&[1]);
        answer.proof.as_mut().unwrap().signature[7] ^= 1;
        assert!(matches!(
            verify_owner(ALICE, answer),
            Err(Error::InvalidSignature { .. } | Error::SignatureVerificationError { .. })
        ));

        let request = owner_request(HEIGHT, ALICE);
        let response = GetOwnerTransactionsInBlockResponse {
            version: Some(get_owner_transactions_in_block_response::Version::V0(
                owner_answer(&[1]),
            )),
        };
        assert!(matches!(
            OwnerTransactionsInBlock::from_proved_response(
                &request,
                &response,
                &TestQuorum::with_another_key()
            ),
            Err(Error::InvalidSignature { .. })
        ));
    }

    #[test]
    fn should_refuse_the_proof_of_another_block_than_the_one_asked_for() {
        let response = GetOwnerTransactionsInBlockResponse {
            version: Some(get_owner_transactions_in_block_response::Version::V0(
                owner_answer(&[1]),
            )),
        };

        let result = OwnerTransactionsInBlock::from_proved_response(
            &owner_request(HEIGHT + 1, ALICE),
            &response,
            &TestQuorum::new(),
        );

        assert_eq!(
            block_proof_error(result),
            BlockProofError::WrongBlock {
                requested: HEIGHT + 1,
                proved: HEIGHT
            }
        );
    }

    #[test]
    fn should_refuse_an_owner_transaction_without_its_paths() {
        let mut answer = owner_answer(&[1, 4]);
        answer.transactions[1].proof = None;

        assert_eq!(
            block_proof_error(verify_owner(ALICE, answer)),
            BlockProofError::MissingTransactionProof {
                height: HEIGHT,
                position: 1
            }
        );
    }

    #[test]
    fn should_refuse_an_owner_transaction_that_is_not_in_the_block() {
        let other = transfer(ALICE, 99).serialize_to_bytes().unwrap();
        let mut answer = owner_answer(&[1]);
        answer.transactions[0].hash = transaction_hash(&other).to_vec();
        answer.transactions[0].state_transition = other;

        assert_eq!(
            block_proof_error(verify_owner(ALICE, answer)),
            BlockProofError::TransactionNotInBlock {
                height: HEIGHT,
                index: 1
            }
        );
    }

    #[test]
    fn should_refuse_an_owner_transaction_with_a_changed_path() {
        let mut answer = owner_answer(&[1]);
        answer.transactions[0]
            .proof
            .as_mut()
            .unwrap()
            .transaction_path[1][0] ^= 1;
        assert_eq!(
            block_proof_error(verify_owner(ALICE, answer)),
            BlockProofError::TransactionNotInBlock {
                height: HEIGHT,
                index: 1
            }
        );

        let mut answer = owner_answer(&[1]);
        answer.transactions[0]
            .proof
            .as_mut()
            .unwrap()
            .result_path
            .pop();
        assert_eq!(
            block_proof_error(verify_owner(ALICE, answer)),
            BlockProofError::MalformedPath { index: 1, total: 5 }
        );
    }

    #[test]
    fn should_refuse_an_owner_transaction_at_another_position() {
        let mut answer = owner_answer(&[1]);
        answer.transactions[0].proof.as_mut().unwrap().index = 0;

        assert_eq!(
            block_proof_error(verify_owner(ALICE, answer)),
            BlockProofError::TransactionNotInBlock {
                height: HEIGHT,
                index: 0
            }
        );
    }

    #[test]
    fn should_refuse_an_owner_transaction_shown_with_another_result() {
        let changes: [fn(&mut OwnerTransactionProto); 3] = [
            |transaction| transaction.code = 0,
            |transaction| transaction.result_data.push(1),
            |transaction| transaction.gas_used += 1,
        ];
        for change in changes {
            let mut answer = owner_answer(&[1]);
            change(&mut answer.transactions[0]);

            assert_eq!(
                block_proof_error(verify_owner(ALICE, answer)),
                BlockProofError::ResultNotInBlock {
                    height: HEIGHT,
                    index: 1
                }
            );
        }
    }

    #[test]
    fn should_refuse_an_owner_transaction_with_a_hash_that_is_not_its_own() {
        let mut answer = owner_answer(&[1]);
        answer.transactions[0].hash[0] ^= 1;

        assert_eq!(
            block_proof_error(verify_owner(ALICE, answer)),
            BlockProofError::TransactionHashMismatch {
                height: HEIGHT,
                position: 0
            }
        );
    }

    /// Both are really in the block, so only the owner check refuses them.
    #[test]
    fn should_refuse_a_transaction_of_the_block_that_is_not_the_owners() {
        // Bob's, then the shielded transfer, which is no one's
        for other in [3, 2] {
            assert_eq!(
                block_proof_error(verify_owner(ALICE, owner_answer(&[1, other]))),
                BlockProofError::NotTheOwners {
                    height: HEIGHT,
                    position: 1
                }
            );
        }
    }

    #[test]
    fn should_refuse_owner_transactions_out_of_block_order() {
        assert_eq!(
            block_proof_error(verify_owner(ALICE, owner_answer(&[4, 1]))),
            BlockProofError::NotInBlockOrder { height: HEIGHT }
        );
        assert_eq!(
            block_proof_error(verify_owner(ALICE, owner_answer(&[1, 1]))),
            BlockProofError::NotInBlockOrder { height: HEIGHT }
        );
    }

    #[test]
    fn should_refuse_owner_transactions_that_disagree_on_the_size_of_the_block() {
        let mut answer = owner_answer(&[1, 4]);
        answer.transactions[1].proof.as_mut().unwrap().total += 1;

        assert_eq!(
            block_proof_error(verify_owner(ALICE, answer)),
            BlockProofError::InconsistentTotal { height: HEIGHT }
        );
    }

    #[test]
    fn should_read_an_unproved_owner_answer_as_the_node_gave_it() {
        let mut answer = owner_answer(&[1, 3]);
        answer.proof = None;
        for transaction in &mut answer.transactions {
            transaction.proof = None;
        }

        let read = OwnerTransactionsInBlock::from_unproved_response(
            &owner_request(HEIGHT, ALICE),
            &GetOwnerTransactionsInBlockResponse {
                version: Some(get_owner_transactions_in_block_response::Version::V0(
                    answer,
                )),
            },
            PlatformVersion::latest(),
        )
        .unwrap();

        assert_eq!(
            read.transactions,
            [block_transaction(1), block_transaction(3)]
        );
    }

    fn blocks_request(start_height: u64, count: u32) -> GetBlocksRequest {
        GetBlocksRequest {
            version: Some(get_blocks_request::Version::V0(GetBlocksRequestV0 {
                start_height,
                count,
                prove: true,
            })),
        }
    }

    /// The block at [`HEIGHT`] with the test transactions, then an empty block.
    fn two_blocks() -> Vec<BlockProto> {
        vec![
            block_answer(HEIGHT, &block_transactions()),
            block_answer(HEIGHT + 1, &[]),
        ]
    }

    fn verify_blocks(count: u32, blocks: Vec<BlockProto>) -> Result<Blocks, Error> {
        Blocks::from_proved_response(
            &blocks_request(HEIGHT, count),
            &GetBlocksResponse {
                version: Some(get_blocks_response::Version::V0(GetBlocksResponseV0 {
                    blocks,
                })),
            },
            &TestQuorum::new(),
        )
    }

    #[test]
    fn should_return_proved_blocks_with_their_transactions() {
        let quorum = TestQuorum::new();
        let answered = two_blocks();
        let first = answered[0].clone();

        let Blocks(blocks) = Blocks::from_proved_response(
            &blocks_request(HEIGHT, 10),
            &GetBlocksResponse {
                version: Some(get_blocks_response::Version::V0(GetBlocksResponseV0 {
                    blocks: answered,
                })),
            },
            &quorum,
        )
        .unwrap();

        assert_eq!(blocks.len(), 2);
        assert_eq!(
            blocks[0],
            Block {
                height: HEIGHT,
                hash: first.hash.try_into().unwrap(),
                time_ms: first.time_ms,
                core_chain_locked_height: CORE_HEIGHT,
                app_hash: [7; 32],
                proposer_pro_tx_hash: [10; 32],
                transactions: (0..5).map(block_transaction).collect(),
            }
        );
        assert_eq!(blocks[1].height, HEIGHT + 1);
        assert!(blocks[1].transactions.is_empty());
        assert_eq!(quorum.verified.lock().unwrap().len(), 2);
    }

    #[test]
    fn should_accept_fewer_blocks_than_asked_for() {
        assert_eq!(verify_blocks(10, Vec::new()).unwrap(), Blocks(Vec::new()));
    }

    #[test]
    fn should_refuse_more_blocks_than_asked_for() {
        assert_eq!(
            block_proof_error(verify_blocks(1, two_blocks())),
            BlockProofError::TooManyBlocks {
                requested: 1,
                returned: 2
            }
        );
    }

    #[test]
    fn should_refuse_a_block_without_its_proof() {
        let mut blocks = two_blocks();
        blocks[1].proof = None;

        assert!(matches!(
            verify_blocks(2, blocks),
            Err(Error::NoProofInResult)
        ));
    }

    #[test]
    fn should_refuse_a_block_the_quorum_did_not_sign() {
        let mut blocks = two_blocks();
        blocks[1].proof.as_mut().unwrap().round += 1;

        assert!(matches!(
            verify_blocks(2, blocks),
            Err(Error::InvalidSignature { .. })
        ));
    }

    #[test]
    fn should_refuse_blocks_that_do_not_follow_the_start_height() {
        let mut blocks = two_blocks();
        blocks.swap(0, 1);
        assert_eq!(
            block_proof_error(verify_blocks(2, blocks)),
            BlockProofError::WrongBlock {
                requested: HEIGHT,
                proved: HEIGHT + 1
            }
        );

        // a gap after the first block
        let blocks = vec![two_blocks().remove(0), block_answer(HEIGHT + 2, &[])];
        assert_eq!(
            block_proof_error(verify_blocks(2, blocks)),
            BlockProofError::WrongBlock {
                requested: HEIGHT + 1,
                proved: HEIGHT + 2
            }
        );
    }

    #[test]
    fn should_refuse_a_block_that_says_something_else_than_its_proved_header() {
        type Change = (&'static str, fn(&mut BlockProto));
        let changes: [Change; 6] = [
            ("height", |block| block.height += 1),
            ("hash", |block| block.hash[0] ^= 1),
            ("time", |block| block.time_ms += 1),
            ("core chain locked height", |block| {
                block.core_chain_locked_height += 1
            }),
            ("app hash", |block| block.app_hash[0] ^= 1),
            ("proposer", |block| block.proposer_pro_tx_hash[0] ^= 1),
        ];
        for (field, change) in changes {
            let mut blocks = two_blocks();
            change(&mut blocks[0]);

            assert_eq!(
                block_proof_error(verify_blocks(2, blocks)),
                BlockProofError::FieldMismatch {
                    height: HEIGHT,
                    field
                }
            );
        }
    }

    #[test]
    fn should_refuse_a_block_whose_transactions_are_not_the_ones_committed() {
        let changes: [fn(&mut Vec<TransactionProto>); 4] = [
            |transactions| {
                transactions.pop();
            },
            |transactions| transactions.swap(0, 1),
            |transactions| transactions.push(transactions[0].clone()),
            |transactions| {
                let other = transfer(ALICE, 99).serialize_to_bytes().unwrap();
                transactions[2].hash = transaction_hash(&other).to_vec();
                transactions[2].state_transition = other;
            },
        ];
        for change in changes {
            let mut blocks = two_blocks();
            change(&mut blocks[0].transactions);

            assert_eq!(
                block_proof_error(verify_blocks(2, blocks)),
                BlockProofError::TransactionsRootMismatch { height: HEIGHT }
            );
        }

        // a transaction slipped into a block that holds none
        let mut blocks = two_blocks();
        blocks[1].transactions = blocks[0].transactions[..1].to_vec();
        assert_eq!(
            block_proof_error(verify_blocks(2, blocks)),
            BlockProofError::TransactionsRootMismatch { height: HEIGHT + 1 }
        );
    }

    #[test]
    fn should_refuse_a_block_whose_results_are_not_the_ones_committed() {
        let changes: [fn(&mut TransactionProto); 3] = [
            |transaction| transaction.code += 1,
            |transaction| transaction.result_data.push(1),
            |transaction| transaction.gas_used += 1,
        ];
        for change in changes {
            let mut blocks = two_blocks();
            change(&mut blocks[0].transactions[3]);

            assert_eq!(
                block_proof_error(verify_blocks(2, blocks)),
                BlockProofError::ResultsRootMismatch { height: HEIGHT }
            );
        }
    }

    #[test]
    fn should_refuse_a_block_transaction_with_a_hash_that_is_not_its_own() {
        let mut blocks = two_blocks();
        blocks[0].transactions[2].hash[0] ^= 1;

        assert_eq!(
            block_proof_error(verify_blocks(2, blocks)),
            BlockProofError::TransactionHashMismatch {
                height: HEIGHT,
                position: 2
            }
        );
    }

    /// The header of the block at [`HEIGHT`] that holds `transactions`, executed under
    /// protocol version `version`.
    fn header_under(version: u64, transactions: &[TestTransaction]) -> Header {
        let mut header = header_of(HEIGHT, transactions);
        header.app_version = version;
        header
    }

    /// The verified answer for the whole block [`header_under`] describes.
    fn verify_block_under(version: u64, transactions: &[TestTransaction]) -> Result<Blocks, Error> {
        let header = header_under(version, transactions);
        verify_blocks(1, vec![block_answer_of(QUORUM_HASH, header, transactions)])
    }

    /// The verified answer for the transactions at `indexes` of that block, asked as
    /// Alice's.
    fn verify_owner_under(
        version: u64,
        transactions: &[TestTransaction],
        indexes: &[usize],
    ) -> Result<OwnerTransactionsInBlock, Error> {
        let header = header_under(version, transactions);
        verify_owner(ALICE, owner_answer_of(header, transactions, indexes))
    }

    /// A document batch of Alice's whose data nests past 256 levels: it decodes under
    /// protocol version 12 and is too deep for the versions after it.
    fn deeply_nested_batch() -> TestTransaction {
        let nested = (0..300).fold(Value::Null, |value, _| Value::Array(vec![value]));
        let transition = StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
            owner_id: ALICE,
            transitions: vec![BatchedTransition::Document(DocumentTransition::Create(
                DocumentCreateTransition::V0(DocumentCreateTransitionV0 {
                    base: DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
                        id: Identifier::default(),
                        identity_contract_nonce: 1,
                        document_type_name: "test".to_string(),
                        data_contract_id: Identifier::default(),
                    }),
                    entropy: [0; 32],
                    data: BTreeMap::from([("nested".to_string(), nested)]),
                    prefunded_voting_balance: None,
                }),
            ))],
            ..Default::default()
        }));
        (transition.serialize_to_bytes().unwrap(), 0, Vec::new(), 1)
    }

    /// The header is signed, so the version its transactions decode under is too.
    #[test]
    fn should_decode_proved_transactions_under_the_protocol_version_of_their_block() {
        // Deeply nested values need more stack than libtest's default test thread has
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(|| {
                let transactions = [deeply_nested_batch()];

                let Blocks(blocks) = verify_block_under(12, &transactions).unwrap();
                assert!(matches!(
                    blocks[0].transactions[0].state_transition,
                    StateTransition::Batch(_)
                ));
                let owned = verify_owner_under(12, &transactions, &[0]).unwrap();
                assert_eq!(owned.transactions, blocks[0].transactions);

                // the same bytes are too deep for a block of protocol version 13
                assert!(matches!(
                    verify_block_under(13, &transactions),
                    Err(Error::UnusableVerifiedResult { .. })
                ));
                assert!(matches!(
                    verify_owner_under(13, &transactions, &[0]),
                    Err(Error::UnusableVerifiedResult { .. })
                ));
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn should_read_a_block_of_a_newer_protocol_version_with_the_newest_rules() {
        let newer = u64::from(PlatformVersion::latest().protocol_version) + 1_000;

        let Blocks(blocks) = verify_block_under(newer, &block_transactions()).unwrap();

        assert_eq!(blocks[0].transactions.len(), 5);
    }

    /// The quorum signed the block and the bytes are in it, so the node told the truth
    /// and every other node would answer the same.
    #[test]
    fn should_not_call_a_proof_invalid_when_a_committed_transaction_does_not_decode() {
        let latest = u64::from(PlatformVersion::latest().protocol_version);
        let mut transactions = block_transactions();
        transactions[1].0 = vec![0xff; 40];

        match verify_block_under(latest, &transactions) {
            Err(Error::UnusableVerifiedResult { reason }) => {
                assert!(reason.contains("transaction 1 of block 7"), "{reason}")
            }
            other => panic!("expected an unusable verified result, got {other:?}"),
        }
        match verify_owner_under(latest, &transactions, &[1]) {
            Err(Error::UnusableVerifiedResult { reason }) => {
                assert!(reason.contains("transaction 0 of block 7"), "{reason}")
            }
            other => panic!("expected an unusable verified result, got {other:?}"),
        }

        // without a proof nothing vouches for the bytes: the answer does not decode
        let mut block = block_answer(HEIGHT, &transactions);
        block.proof = None;
        assert!(matches!(
            Blocks::from_unproved_response(
                &blocks_request(HEIGHT, 1),
                &GetBlocksResponse {
                    version: Some(get_blocks_response::Version::V0(GetBlocksResponseV0 {
                        blocks: vec![block],
                    })),
                },
                PlatformVersion::latest(),
            ),
            Err(Error::ProtocolError { .. })
        ));
    }

    #[test]
    fn should_not_call_a_proof_invalid_when_this_build_cannot_read_the_blocks_version() {
        assert!(matches!(
            verify_block_under(0, &block_transactions()),
            Err(Error::UnusableVerifiedResult { .. })
        ));
    }

    #[test]
    fn should_read_unproved_blocks_as_the_node_gave_them() {
        let mut answered = two_blocks();
        for block in &mut answered {
            block.proof = None;
        }
        let response = |blocks| GetBlocksResponse {
            version: Some(get_blocks_response::Version::V0(GetBlocksResponseV0 {
                blocks,
            })),
        };

        let Blocks(blocks) = Blocks::from_unproved_response(
            &blocks_request(HEIGHT, 2),
            &response(answered.clone()),
            PlatformVersion::latest(),
        )
        .unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].transactions[4], block_transaction(4));

        // held to the heights that were asked for, and to no more blocks than that
        assert!(matches!(
            Blocks::from_unproved_response(
                &blocks_request(HEIGHT + 1, 2),
                &response(answered.clone()),
                PlatformVersion::latest(),
            ),
            Err(Error::ResponseDecodeError { .. })
        ));
        assert!(matches!(
            Blocks::from_unproved_response(
                &blocks_request(HEIGHT, 1),
                &response(answered),
                PlatformVersion::latest(),
            ),
            Err(Error::ResponseDecodeError { .. })
        ));
    }
}
