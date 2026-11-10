//! The two block queries ([`crate::platform::blocks`]) against a mock node that answers
//! with blocks a test quorum signed.

use super::SdkInstance;
use crate::platform::blocks::{Blocks, BlocksQuery, OwnerTransactionsInBlockQuery};
use crate::platform::{Identifier, Query};
use crate::{Error, Sdk, SdkBuilder};
use dapi_grpc::mock::Mockable;
use dapi_grpc::platform::v0::get_blocks_request::GetBlocksRequestV0;
use dapi_grpc::platform::v0::get_blocks_response::{self, GetBlocksResponseV0};
use dapi_grpc::platform::v0::get_owner_transactions_in_block_request::GetOwnerTransactionsInBlockRequestV0;
use dapi_grpc::platform::v0::get_owner_transactions_in_block_response;
use dapi_grpc::platform::v0::{
    get_blocks_request, get_owner_transactions_in_block_request, GetBlocksRequest,
    GetBlocksResponse, GetOwnerTransactionsInBlockRequest, GetOwnerTransactionsInBlockResponse,
};
use dash_context_provider::ContextProvider;
use dash_context_provider::{ContextProviderError, QuorumKeyFuture};
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{CoreBlockHeight, DataContract};
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use drive_proof_verifier::block_proof::fixtures::{
    block_answer, block_answer_by, block_answer_of, header_of, owner_answer, quorum_public_key,
    TestQuorum, TestTransaction, CORE_HEIGHT, QUORUM_HASH, QUORUM_TYPE,
};
use drive_proof_verifier::block_proof::BlockProofError;
use rs_dapi_client::transport::TransportRequest;
use rs_dapi_client::{Address, CanRetry, ExecutionResponse, RequestSettings};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

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

/// The transitions of the test block: Alice's is at position 1.
fn transitions() -> Vec<StateTransition> {
    vec![transfer(BOB, 10), transfer(ALICE, 11), transfer(BOB, 12)]
}

fn block_transactions() -> Vec<TestTransaction> {
    transitions()
        .iter()
        .zip(0u32..)
        .map(|(transition, index)| {
            (
                transition
                    .serialize_to_bytes()
                    .expect("a transfer serializes"),
                index * 100,
                vec![index as u8],
                i64::from(index) + 1,
            )
        })
        .collect()
}

fn blocks_response(
    blocks: Vec<get_blocks_response::get_blocks_response_v0::Block>,
) -> GetBlocksResponse {
    GetBlocksResponse {
        version: Some(get_blocks_response::Version::V0(GetBlocksResponseV0 {
            blocks,
        })),
    }
}

/// A mock SDK that answers the request `query` encodes with `response`, and verifies
/// with the keys of `provider`.
async fn sdk_answering<R, Q>(
    builder: SdkBuilder,
    query: &Q,
    response: R::Response,
    provider: impl ContextProvider + 'static,
) -> Sdk
where
    R: TransportRequest + Mockable,
    R::Response: Mockable,
    Q: Query<R>,
{
    let sdk = builder
        .with_version(PlatformVersion::latest())
        .build()
        .expect("build a mock SDK");
    sdk.set_context_provider(provider)
        .expect("the test provider states no requirements");
    let request = query
        .query(&sdk.query_settings())
        .expect("encode the request");
    let SdkInstance::Mock { dapi, .. } = &sdk.inner else {
        panic!("mock SDK");
    };
    let address: Address = "http://127.0.0.1:9000"
        .parse()
        .expect("mock responder address");
    dapi.lock()
        .await
        .expect(
            &request,
            &Ok(ExecutionResponse {
                inner: response,
                address,
                retries: 0,
            }),
        )
        .expect("register the mock response");
    sdk
}

fn no_retries() -> RequestSettings {
    RequestSettings {
        retries: Some(0),
        ..Default::default()
    }
}

fn block_proof_error(error: Error) -> BlockProofError {
    match error {
        Error::Proof(drive_proof_verifier::Error::InvalidBlockProof(error)) => error,
        other => panic!("expected a refused block proof, got {other:?}"),
    }
}

#[test]
fn should_ask_for_proofs_as_the_sdk_is_set_up() {
    let sdk = Sdk::new_mock();
    let proved = sdk.query_settings();
    let unproved = proved.without_proofs();
    let owner = OwnerTransactionsInBlockQuery {
        height: HEIGHT,
        owner_id: ALICE,
    };
    let blocks = BlocksQuery {
        start_height: HEIGHT,
        count: 3,
    };

    for (settings, prove) in [(proved, true), (unproved, false)] {
        assert_eq!(
            owner.query(&settings).expect("encode"),
            GetOwnerTransactionsInBlockRequest {
                version: Some(get_owner_transactions_in_block_request::Version::V0(
                    GetOwnerTransactionsInBlockRequestV0 {
                        height: HEIGHT,
                        owner_id: ALICE.to_vec(),
                        prove,
                    }
                )),
            }
        );
        assert_eq!(
            blocks.query(&settings).expect("encode"),
            GetBlocksRequest {
                version: Some(get_blocks_request::Version::V0(GetBlocksRequestV0 {
                    start_height: HEIGHT,
                    count: 3,
                    prove,
                })),
            }
        );
    }
}

/// The blocks are far below the trusted height and years old: a block is what it is
/// at any age, so no freshness tolerance applies to it.
#[tokio::test]
async fn should_fetch_proved_blocks_however_old() {
    let query = BlocksQuery {
        start_height: HEIGHT,
        count: 10,
    };
    let response = blocks_response(vec![
        block_answer(HEIGHT, &block_transactions()),
        block_answer(HEIGHT + 1, &[]),
    ]);
    let builder = SdkBuilder::new_mock()
        .with_height_tolerance(Some(1))
        .with_trusted_initial_height(1_000_000);
    let sdk = sdk_answering(builder, &query, response, TestQuorum::new()).await;

    let Blocks(blocks) = sdk.fetch_blocks(query).await.expect("verified blocks");

    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].height, HEIGHT);
    assert_eq!(
        blocks[0]
            .transactions
            .iter()
            .map(|transaction| transaction.state_transition.clone())
            .collect::<Vec<_>>(),
        transitions()
    );
    assert_eq!(blocks[0].transactions[2].code, 200);
    assert!(blocks[1].transactions.is_empty());
}

#[tokio::test]
async fn should_refuse_blocks_whose_contents_their_signed_header_does_not_commit_to() {
    let query = BlocksQuery {
        start_height: HEIGHT,
        count: 1,
    };
    let mut block = block_answer(HEIGHT, &block_transactions());
    block.transactions[1].code = 0;
    let sdk = sdk_answering(
        SdkBuilder::new_mock(),
        &query,
        blocks_response(vec![block]),
        TestQuorum::new(),
    )
    .await;

    let error = sdk
        .fetch_blocks_with_settings(query, no_retries())
        .await
        .expect_err("a changed result");

    assert_eq!(
        block_proof_error(error),
        BlockProofError::ResultsRootMismatch { height: HEIGHT }
    );
}

#[tokio::test]
async fn should_refuse_a_block_signed_with_another_key_than_the_quorums() {
    let query = BlocksQuery {
        start_height: HEIGHT,
        count: 1,
    };
    let sdk = sdk_answering(
        SdkBuilder::new_mock(),
        &query,
        blocks_response(vec![block_answer(HEIGHT, &block_transactions())]),
        TestQuorum::with_another_key(),
    )
    .await;

    let error = sdk
        .fetch_blocks_with_settings(query, no_retries())
        .await
        .expect_err("a signature of another key");

    assert!(matches!(
        error,
        Error::Proof(drive_proof_verifier::Error::InvalidSignature { .. })
    ));
}

/// Caches no key until asked to fetch it, and counts the fetches.
#[derive(Clone, Default)]
struct FetchingQuorums {
    cached: Arc<Mutex<Vec<[u8; 32]>>>,
    fetches: Arc<AtomicUsize>,
    /// Quorums the trusted source does not list
    unknown: Vec<[u8; 32]>,
}

impl ContextProvider for FetchingQuorums {
    fn get_data_contract(
        &self,
        _: &Identifier,
        _: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        Ok(None)
    }

    fn get_token_configuration(
        &self,
        _: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(None)
    }

    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        _: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        assert_eq!(quorum_type, QUORUM_TYPE);
        if self
            .cached
            .lock()
            .expect("cache lock")
            .contains(&quorum_hash)
        {
            Ok(quorum_public_key(quorum_hash))
        } else {
            Err(ContextProviderError::InvalidQuorum(
                "not cached".to_string(),
            ))
        }
    }

    fn fetch_quorum_public_key(
        &self,
        _: u32,
        quorum_hash: [u8; 32],
        _: u32,
    ) -> Option<QuorumKeyFuture> {
        self.fetches.fetch_add(1, Ordering::SeqCst);
        if self.unknown.contains(&quorum_hash) {
            return Some(Box::pin(async { Ok(None) }));
        }
        self.cached.lock().expect("cache lock").push(quorum_hash);
        Some(Box::pin(
            async move { Ok(Some(quorum_public_key(quorum_hash))) },
        ))
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(1)
    }
}

/// The validator set rotates every few blocks, so a page of blocks often spans two
/// quorums, and a provider that caches keys may hold neither.
#[tokio::test]
async fn should_fetch_the_key_of_every_quorum_that_signed_a_page_of_blocks() {
    let other_quorum = [5; 32];
    let query = BlocksQuery {
        start_height: HEIGHT,
        count: 3,
    };
    let response = blocks_response(vec![
        block_answer(HEIGHT, &block_transactions()),
        block_answer_by(other_quorum, HEIGHT + 1, &[]),
        block_answer_by(other_quorum, HEIGHT + 2, &[]),
    ]);
    let provider = FetchingQuorums::default();
    let sdk = sdk_answering(SdkBuilder::new_mock(), &query, response, provider.clone()).await;

    let Blocks(blocks) = sdk.fetch_blocks(query).await.expect("verified blocks");

    assert_eq!(blocks.len(), 3);
    assert_eq!(provider.fetches.load(Ordering::SeqCst), 2);
    assert_eq!(
        *provider.cached.lock().expect("cache lock"),
        [QUORUM_HASH, other_quorum]
    );
}

#[tokio::test]
async fn should_refuse_a_block_naming_a_quorum_the_trusted_source_does_not_list() {
    let other_quorum = [5; 32];
    let query = BlocksQuery {
        start_height: HEIGHT,
        count: 2,
    };
    let response = blocks_response(vec![
        block_answer(HEIGHT, &[]),
        block_answer_by(other_quorum, HEIGHT + 1, &[]),
    ]);
    let provider = FetchingQuorums {
        unknown: vec![other_quorum],
        ..Default::default()
    };
    let sdk = sdk_answering(SdkBuilder::new_mock(), &query, response, provider).await;

    let error = sdk
        .fetch_blocks_with_settings(query, no_retries())
        .await
        .expect_err("an unknown quorum");

    assert!(matches!(
        error,
        Error::Proof(drive_proof_verifier::Error::QuorumKeyUnavailable { quorum_hash, .. })
            if quorum_hash == other_quorum
    ));
}

/// How many Core heights past the one a key was fetched for [`HeightBoundQuorums`] serves
/// it at.
const SERVED_HEIGHTS: u32 = 24;

/// A quorum hash and the Core height its key was fetched for.
type FetchedKey = ([u8; 32], u32);

/// Holds a fetched key only for the Core heights near the one it was fetched for, as a
/// provider that proves a quorum's key for a height does.
#[derive(Clone, Default)]
struct HeightBoundQuorums {
    fetched: Arc<Mutex<Vec<FetchedKey>>>,
}

impl ContextProvider for HeightBoundQuorums {
    fn get_data_contract(
        &self,
        _: &Identifier,
        _: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        Ok(None)
    }

    fn get_token_configuration(
        &self,
        _: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(None)
    }

    fn get_quorum_public_key(
        &self,
        _: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        let served = self
            .fetched
            .lock()
            .expect("fetch lock")
            .iter()
            .any(|(hash, height)| {
                *hash == quorum_hash
                    && (*height..=*height + SERVED_HEIGHTS).contains(&core_chain_locked_height)
            });
        if served {
            Ok(quorum_public_key(quorum_hash))
        } else {
            Err(ContextProviderError::InvalidQuorum(
                "not held for this Core height".to_string(),
            ))
        }
    }

    fn fetch_quorum_public_key(
        &self,
        _: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Option<QuorumKeyFuture> {
        self.fetched
            .lock()
            .expect("fetch lock")
            .push((quorum_hash, core_chain_locked_height));
        Some(Box::pin(
            async move { Ok(Some(quorum_public_key(quorum_hash))) },
        ))
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(1)
    }
}

/// A quorum signs for many blocks, and a provider that proves its key for a Core height
/// serves the key only near that height. Two blocks of a page chain locked further apart
/// than that each need the key fetched, and the second miss is no fault of the provider's.
#[tokio::test]
async fn should_fetch_a_quorums_key_again_for_a_core_height_its_fetch_does_not_serve() {
    let later_core_height = CORE_HEIGHT + SERVED_HEIGHTS + 1;
    let mut later = header_of(HEIGHT + 1, &[]);
    later.core_chain_locked_height = later_core_height;
    let query = BlocksQuery {
        start_height: HEIGHT,
        count: 2,
    };
    let response = blocks_response(vec![
        block_answer(HEIGHT, &block_transactions()),
        block_answer_of(QUORUM_HASH, later, &[]),
    ]);
    let provider = HeightBoundQuorums::default();
    let sdk = sdk_answering(SdkBuilder::new_mock(), &query, response, provider.clone()).await;

    let Blocks(blocks) = sdk
        .fetch_blocks_with_settings(query, no_retries())
        .await
        .expect("verified blocks");

    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[1].core_chain_locked_height, later_core_height);
    assert_eq!(
        *provider.fetched.lock().expect("fetch lock"),
        [(QUORUM_HASH, CORE_HEIGHT), (QUORUM_HASH, later_core_height)]
    );
}

/// The block is signed and the bytes are in it, so every node would answer the same: the
/// node is not to blame, and asking another one cannot help.
#[tokio::test]
async fn should_not_blame_a_node_for_a_committed_transaction_that_does_not_decode() {
    let mut transactions = block_transactions();
    transactions[1].0 = vec![0xff; 40];
    let query = BlocksQuery {
        start_height: HEIGHT,
        count: 1,
    };
    let sdk = sdk_answering(
        SdkBuilder::new_mock(),
        &query,
        blocks_response(vec![block_answer(HEIGHT, &transactions)]),
        TestQuorum::new(),
    )
    .await;

    let error = sdk
        .fetch_blocks(query)
        .await
        .expect_err("a transaction this build cannot read");

    assert!(
        matches!(&error, Error::InvalidRequest(reason) if reason.contains("transaction 1 of block 7")),
        "got {error:?}"
    );
    assert!(!error.can_retry());
}

#[tokio::test]
async fn should_fetch_the_proved_transactions_of_an_owner() {
    let query = OwnerTransactionsInBlockQuery {
        height: HEIGHT,
        owner_id: ALICE,
    };
    let response = GetOwnerTransactionsInBlockResponse {
        version: Some(get_owner_transactions_in_block_response::Version::V0(
            owner_answer(HEIGHT, &block_transactions(), &[1]),
        )),
    };
    let sdk = sdk_answering(SdkBuilder::new_mock(), &query, response, TestQuorum::new()).await;

    let answer = sdk
        .fetch_owner_transactions_in_block(query)
        .await
        .expect("verified transactions");

    assert_eq!(answer.height, HEIGHT);
    assert_eq!(answer.transactions.len(), 1);
    assert_eq!(answer.transactions[0].state_transition, transfer(ALICE, 11));
    assert_eq!(answer.transactions[0].code, 100);
    assert_eq!(answer.transactions[0].result_data, [1]);
    assert_eq!(answer.transactions[0].gas_used, 2);
}

#[tokio::test]
async fn should_refuse_a_transaction_of_the_block_that_another_identity_owns() {
    let query = OwnerTransactionsInBlockQuery {
        height: HEIGHT,
        owner_id: ALICE,
    };
    let response = GetOwnerTransactionsInBlockResponse {
        version: Some(get_owner_transactions_in_block_response::Version::V0(
            owner_answer(HEIGHT, &block_transactions(), &[1, 2]),
        )),
    };
    let sdk = sdk_answering(SdkBuilder::new_mock(), &query, response, TestQuorum::new()).await;

    let error = sdk
        .fetch_owner_transactions_in_block_with_settings(query, no_retries())
        .await
        .expect_err("a transaction of Bob's");

    assert_eq!(
        block_proof_error(error),
        BlockProofError::NotTheOwners {
            height: HEIGHT,
            position: 1
        }
    );
}

#[tokio::test]
async fn should_refuse_a_proved_answer_that_carries_no_proof() {
    let query = OwnerTransactionsInBlockQuery {
        height: HEIGHT,
        owner_id: ALICE,
    };
    let mut answer = owner_answer(HEIGHT, &block_transactions(), &[1]);
    answer.proof = None;
    let response = GetOwnerTransactionsInBlockResponse {
        version: Some(get_owner_transactions_in_block_response::Version::V0(
            answer,
        )),
    };
    let sdk = sdk_answering(SdkBuilder::new_mock(), &query, response, TestQuorum::new()).await;

    let error = sdk
        .fetch_owner_transactions_in_block_with_settings(query, no_retries())
        .await
        .expect_err("no proof");

    assert!(matches!(
        error,
        Error::Proof(drive_proof_verifier::Error::NoProofInResult)
    ));
}

/// With proofs off the answer is the node's word: nothing is checked against a
/// quorum, so even a transaction of another owner passes.
#[tokio::test]
async fn should_read_an_answer_without_verifying_it_when_proofs_are_off() {
    let query = OwnerTransactionsInBlockQuery {
        height: HEIGHT,
        owner_id: ALICE,
    };
    let mut answer = owner_answer(HEIGHT, &block_transactions(), &[0, 1]);
    answer.proof = None;
    for transaction in &mut answer.transactions {
        transaction.proof = None;
    }
    let response = GetOwnerTransactionsInBlockResponse {
        version: Some(get_owner_transactions_in_block_response::Version::V0(
            answer,
        )),
    };
    let builder = SdkBuilder::new_mock().with_proofs(false);
    let sdk = sdk_answering(builder, &query, response, TestQuorum::with_another_key()).await;

    let answer = sdk
        .fetch_owner_transactions_in_block(query)
        .await
        .expect("an unverified answer");

    assert_eq!(answer.transactions.len(), 2);
    assert_eq!(answer.transactions[0].state_transition, transfer(BOB, 10));
}
