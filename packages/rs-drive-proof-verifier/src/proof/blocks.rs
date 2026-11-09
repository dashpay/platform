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

/// The protocol version a block executed under, which decides what decodes in that block.
/// A block of a version newer than any this build knows is read with the newest rules
/// there are.
fn block_platform_version(header: &Header) -> Result<&'static PlatformVersion, Error> {
    let latest = PlatformVersion::latest();
    match u32::try_from(header.app_version) {
        Ok(version) if version <= latest.protocol_version => Ok(PlatformVersion::get(version)?),
        _ => Ok(latest),
    }
}

/// A transaction as an answer carries it.
struct AnsweredTransaction {
    state_transition: Vec<u8>,
    hash: Vec<u8>,
    code: u32,
    result_data: Vec<u8>,
    gas_used: i64,
}

impl From<OwnerTransactionProto> for AnsweredTransaction {
    fn from(transaction: OwnerTransactionProto) -> Self {
        Self {
            state_transition: transaction.state_transition,
            hash: transaction.hash,
            code: transaction.code,
            result_data: transaction.result_data,
            gas_used: transaction.gas_used,
        }
    }
}

impl From<TransactionProto> for AnsweredTransaction {
    fn from(transaction: TransactionProto) -> Self {
        Self {
            state_transition: transaction.state_transition,
            hash: transaction.hash,
            code: transaction.code,
            result_data: transaction.result_data,
            gas_used: transaction.gas_used,
        }
    }
}

impl AnsweredTransaction {
    /// The hash Tenderdash knows the transaction by, which must be the one it came with.
    /// `position` is the transaction's place in the answer.
    fn hash(&self, height: u64, position: usize) -> Result<[u8; 32], BlockProofError> {
        let hash = transaction_hash(&self.state_transition);
        if hash.as_slice() != self.hash {
            return Err(BlockProofError::TransactionHashMismatch { height, position });
        }
        Ok(hash)
    }

    fn result_leaf(&self) -> Vec<u8> {
        result_leaf(self.code, &self.result_data, self.gas_used)
    }

    /// Decode the transaction as Drive decoded it, under the protocol version of its
    /// block. `hash` is what [`Self::hash`] returned.
    fn decode(
        self,
        height: u64,
        position: usize,
        hash: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> Result<BlockTransaction, Error> {
        let state_transition = StateTransition::deserialize_from_bytes_untrusted_in_version(
            &self.state_transition,
            platform_version,
        )
        .map_err(|error| Error::ProtocolError {
            error: format!("transaction {position} of block {height} does not decode: {error}"),
        })?;

        Ok(BlockTransaction {
            state_transition,
            hash,
            code: self.code,
            result_data: self.result_data,
            gas_used: self.gas_used,
        })
    }

    /// Decode the transaction of an unproved answer, under a protocol version the caller
    /// chose. Its hash is computed here: the one it came with is the node's word.
    fn decode_unproved(
        self,
        height: u64,
        position: usize,
        platform_version: &PlatformVersion,
    ) -> Result<BlockTransaction, Error> {
        let hash = transaction_hash(&self.state_transition);
        self.decode(height, position, hash, platform_version)
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
    /// See [`OwnerTransactionsInBlock`] for what the proof leaves open.
    pub fn from_proved_response(
        request: &GetOwnerTransactionsInBlockRequest,
        response: GetOwnerTransactionsInBlockResponse,
        provider: &dyn ContextProvider,
    ) -> Result<Self, Error> {
        let (height, owner_id) = owner_request(request)?;
        let get_owner_transactions_in_block_response::Version::V0(response) =
            response.version.ok_or(Error::EmptyVersion)?;
        let proof = response.proof.ok_or(Error::NoProofInResult)?;

        let header = verify_block_proof(&proof, provider)?;
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
            .into_iter()
            .enumerate()
            .map(|(position, mut transaction)| {
                let proof = transaction
                    .proof
                    .take()
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
                verify_transaction_proof(header, &hash, &transaction.result_leaf(), &proof)?;

                let transaction = transaction.decode(height, position, hash, platform_version)?;
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
        response: GetOwnerTransactionsInBlockResponse,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let (height, _) = owner_request(request)?;
        let get_owner_transactions_in_block_response::Version::V0(response) =
            response.version.ok_or(Error::EmptyVersion)?;

        let transactions = response
            .transactions
            .into_iter()
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
fn blocks_of_response(
    request: &GetBlocksRequest,
    response: GetBlocksResponse,
) -> Result<(u64, u32, Vec<BlockProto>), Error> {
    let Some(get_blocks_request::Version::V0(request)) = &request.version else {
        return Err(Error::EmptyVersion);
    };
    let get_blocks_response::Version::V0(response) = response.version.ok_or(Error::EmptyVersion)?;
    Ok((request.start_height, request.count, response.blocks))
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
    mut block: BlockProto,
    provider: &dyn ContextProvider,
) -> Result<Block, Error> {
    let proof = block.proof.take().ok_or(Error::NoProofInResult)?;
    let header = verify_block_proof(&proof, provider)?;
    if header.height != height {
        return Err(wrong_block(height, header.height));
    }

    // What the answer says about the block next to the proof must be what the proof says
    check_block_fields(&proof, &block)?;

    // A block holds all its transactions, so the two roots are recomputed from them
    let transactions = std::mem::take(&mut block.transactions)
        .into_iter()
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
        .into_iter()
        .zip(hashes)
        .enumerate()
        .map(|(position, (transaction, hash))| {
            transaction.decode(height, position, hash, platform_version)
        })
        .collect::<Result<Vec<_>, Error>>()?;

    block_summary(height, &block, transactions)
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
    /// See [`Blocks`] for what the proofs leave open.
    pub fn from_proved_response(
        request: &GetBlocksRequest,
        response: GetBlocksResponse,
        provider: &dyn ContextProvider,
    ) -> Result<Self, Error> {
        let (start_height, count, blocks) = blocks_of_response(request, response)?;
        if more_than_asked(count, &blocks) {
            return Err(BlockProofError::TooManyBlocks {
                requested: count,
                returned: blocks.len(),
            }
            .into());
        }
        blocks
            .into_iter()
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
        response: GetBlocksResponse,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let (start_height, count, blocks) = blocks_of_response(request, response)?;
        if more_than_asked(count, &blocks) {
            return Err(Error::ResponseDecodeError {
                error: format!("asked for {count} blocks but {} came back", blocks.len()),
            });
        }
        blocks
            .into_iter()
            .enumerate()
            .map(|(position, mut block)| {
                let height = expected_height(start_height, position)?;
                if block.height != height {
                    return Err(Error::ResponseDecodeError {
                        error: format!(
                            "block {position} of the answer is at height {}, not {height}",
                            block.height
                        ),
                    });
                }
                let transactions = std::mem::take(&mut block.transactions)
                    .into_iter()
                    .enumerate()
                    .map(|(position, transaction)| {
                        AnsweredTransaction::from(transaction).decode_unproved(
                            height,
                            position,
                            platform_version,
                        )
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                block_summary(height, &block, transactions)
            })
            .collect::<Result<Vec<_>, Error>>()
            .map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_proof::fixtures::{
        self, block_answer, header_of, signed_proof, TestQuorum, TestTransaction, CORE_HEIGHT,
    };
    use dapi_grpc::platform::v0::get_blocks_request::GetBlocksRequestV0;
    use dapi_grpc::platform::v0::get_blocks_response::GetBlocksResponseV0;
    use dapi_grpc::platform::v0::get_owner_transactions_in_block_request::GetOwnerTransactionsInBlockRequestV0;
    use dapi_grpc::platform::v0::get_owner_transactions_in_block_response::GetOwnerTransactionsInBlockResponseV0;
    use dpp::serialization::PlatformSerializable;
    use dpp::shielded::SerializedAction;
    use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
    use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
    use dpp::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;
    use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;

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
            GetOwnerTransactionsInBlockResponse {
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
                response,
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
            response,
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
            GetOwnerTransactionsInBlockResponse {
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
            GetBlocksResponse {
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
            GetBlocksResponse {
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

    /// The header is signed, so the version its transactions decode under is too.
    #[test]
    fn should_read_a_block_of_a_newer_protocol_version_with_the_newest_rules() {
        let transactions = block_transactions();
        let mut header = header_of(HEIGHT, &transactions);
        header.app_version = u64::from(PlatformVersion::latest().protocol_version) + 1_000;
        let mut block = block_answer(HEIGHT, &transactions);
        let proof = signed_proof(header);
        block.hash = proof.block_id.as_ref().unwrap().hash.clone();
        block.proof = Some(proof);

        let Blocks(blocks) = verify_blocks(1, vec![block]).unwrap();

        assert_eq!(blocks[0].transactions.len(), 5);
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
            response(answered.clone()),
            PlatformVersion::latest(),
        )
        .unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].transactions[4], block_transaction(4));

        // held to the heights that were asked for, and to no more blocks than that
        assert!(matches!(
            Blocks::from_unproved_response(
                &blocks_request(HEIGHT + 1, 2),
                response(answered.clone()),
                PlatformVersion::latest(),
            ),
            Err(Error::ResponseDecodeError { .. })
        ));
        assert!(matches!(
            Blocks::from_unproved_response(
                &blocks_request(HEIGHT, 1),
                response(answered),
                PlatformVersion::latest(),
            ),
            Err(Error::ResponseDecodeError { .. })
        ));
    }
}
