//! A node's signed answer to a SUM or AVG documents query verifies through the SDK's
//! `FromProof<DocumentQuery>` impls with the whole total, past 64 bits from protocol 15.
//!
//! The node is this Drive with the document history contract, holding listings recorded for
//! two source contracts: contract A at prices whose total passes `i64::MAX`, contract B at
//! small ones. The node answers the v1 `getDocuments` request the SDK encodes through the
//! query handler, and the test quorum signs the proof over the node's root. Protocol 15
//! keeps listed prices in 128-bit trees; protocol 14 keeps them in 64 bits and still
//! answers a narrow total the same way.
#[allow(dead_code)]
mod signed_response;

use dapi_grpc::platform::v0::get_documents_response::{
    get_documents_response_v1, GetDocumentsResponseV1, Version as ResponseVersion,
};
use dapi_grpc::platform::v0::{GetDocumentsRequest, GetDocumentsResponse, ResponseMetadata};
use dash_platform_queries::documents::document_query::DocumentQuery;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::Network;
use dpp::data_contract::DataContract;
use dpp::document::document_event::DocumentEvent;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::prelude::IdentityNonce;
use dpp::version::{PlatformVersion, TryFromPlatformVersioned};
use drive::query::projection::SelectProjection;
use drive::query::{WhereClause, WhereOperator};
use drive_abci::config::PlatformConfig;
use drive_abci::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
use drive_abci::rpc::core::MockCoreRPCLike;
use drive_abci::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use drive_proof_verifier::{
    DocumentAverage, DocumentSplitAverages, DocumentSplitSums, DocumentSum, FromProof,
    SplitAverageEntry, SplitSumEntry,
};
use signed_response::{signed, Provider};
use std::sync::Arc;

const CONTRACT_A: [u8; 32] = [0x0A; 32];
const CONTRACT_B: [u8; 32] = [0x0B; 32];
const DOCUMENT_TYPE: &str = "priceUpdate";
const HIGHEST: u64 = i64::MAX as u64;
/// Contract A's listings: their total passes `i64::MAX`, and their average is not whole.
const PRICES_A: [u64; 3] = [HIGHEST, HIGHEST, HIGHEST - 1];
/// Contract B's listings.
const PRICES_B: [u64; 2] = [5, 70_000];

fn protocol_15() -> &'static PlatformVersion {
    let latest = PlatformVersion::latest();
    if latest.protocol_version >= 15 {
        latest
    } else {
        PlatformVersion::get(15).expect("protocol 15")
    }
}

fn protocol_14() -> &'static PlatformVersion {
    PlatformVersion::get(14).expect("protocol 14")
}

fn total(prices: &[u64]) -> i128 {
    prices.iter().map(|price| i128::from(*price)).sum()
}

/// A node at one protocol version holding the document history contract.
struct Node {
    platform: TempPlatform<MockCoreRPCLike>,
    state: Arc<PlatformState>,
    version: &'static PlatformVersion,
    contract: Arc<DataContract>,
    next_nonce: IdentityNonce,
}

impl Node {
    fn new(version: &'static PlatformVersion) -> Self {
        let platform = TestPlatformBuilder::new()
            .with_config(PlatformConfig::default_for_network(Network::Testnet))
            .with_initial_protocol_version(version.protocol_version)
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let state = platform.platform.state.load_full();
        assert_eq!(
            state.current_protocol_version_in_consensus(),
            version.protocol_version
        );
        let drive = &platform.platform.drive;
        let contract = drive
            .cache
            .system_data_contracts
            .load_document_history(version)
            .expect("the document history contract");
        drive
            .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
            .expect("store the document history contract");
        Self {
            platform,
            state,
            version,
            contract,
            next_nonce: 1,
        }
    }

    /// Records a listing of a document of `source_contract` at `price`, at its own block
    /// time so every record has its own `$createdAt`.
    fn list(&mut self, source_contract: [u8; 32], price: u64) {
        let nonce = self.next_nonce;
        self.next_nonce += 1;
        let drive = &self.platform.platform.drive;
        let operations = drive
            .add_document_history_operations(
                Identifier::new(source_contract),
                "card",
                Identifier::new([nonce as u8; 32]),
                Identifier::new([1; 32]),
                nonce,
                DocumentEvent::PriceUpdate { price },
                &BlockInfo::default_with_time(1_000 * nonce),
                &mut None,
                None,
                self.version,
            )
            .expect("build the listing record");
        drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                operations,
                &mut vec![],
                &self.version.drive,
            )
            .expect("record the listing");
    }

    fn root(&self) -> [u8; 32] {
        self.platform
            .platform
            .drive
            .grove
            .root_hash(None, &self.version.drive.grove_version)
            .unwrap()
            .expect("root")
    }

    /// The query handler's proved answer to `query` as the SDK sends it, signed by the test
    /// quorum over this node's root at this node's protocol version.
    fn signed_answer(&self, query: &DocumentQuery) -> GetDocumentsResponse {
        let request = GetDocumentsRequest::try_from_platform_versioned(query.clone(), self.version)
            .expect("encode the query");
        let result = self
            .platform
            .platform
            .query_documents(request, &self.state, self.version)
            .expect("query");
        assert!(
            result.errors.is_empty(),
            "query errors: {:?}",
            result.errors
        );
        let response = result.data.expect("response");
        let Some(ResponseVersion::V1(GetDocumentsResponseV1 {
            result: Some(get_documents_response_v1::Result::Proof(proof)),
            metadata: Some(node_metadata),
        })) = response.version
        else {
            panic!("a proved v1 answer")
        };
        assert_eq!(
            node_metadata.protocol_version,
            self.version.protocol_version
        );
        let metadata = ResponseMetadata {
            height: 10,
            core_chain_locked_height: 1,
            time_ms: 1_700_000_000_000,
            protocol_version: self.version.protocol_version,
            chain_id: "dash-testnet-51".into(),
            ..Default::default()
        };
        GetDocumentsResponse {
            version: Some(ResponseVersion::V1(GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Proof(signed(
                    proof.grovedb_proof,
                    &metadata,
                    self.root(),
                ))),
                metadata: Some(metadata),
            })),
        }
    }

    /// `select` of the listings of the source contracts `in_contracts` recorded in the time
    /// window: one contract by equality, several by `in`, grouped by the contract.
    fn query(&self, select: SelectProjection, in_contracts: &[[u8; 32]]) -> DocumentQuery {
        let contract_clause = match in_contracts {
            [single] => WhereClause {
                field: "dataContractId".into(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(*single),
            },
            several => WhereClause {
                field: "dataContractId".into(),
                operator: WhereOperator::In,
                value: Value::Array(several.iter().copied().map(Value::Identifier).collect()),
            },
        };
        let query = DocumentQuery::new(self.contract.clone(), DOCUMENT_TYPE)
            .expect("the priceUpdate type")
            .with_select(select)
            .with_where(contract_clause)
            .with_where(WhereClause {
                field: "$createdAt".into(),
                operator: WhereOperator::Between,
                value: Value::Array(vec![Value::U64(0), Value::U64(1_000_000)]),
            });
        if in_contracts.len() > 1 {
            query.with_group_by("dataContractId")
        } else {
            query
        }
    }

    fn verify<
        T: FromProof<DocumentQuery, Request = DocumentQuery, Response = GetDocumentsResponse>,
    >(
        &self,
        query: DocumentQuery,
    ) -> T {
        let answer = self.signed_answer(&query);
        T::maybe_from_proof_with_metadata(query, answer, Network::Testnet, self.version, &Provider)
            .map(|(value, _, _)| value)
            .expect("the signed answer verifies")
            .expect("a proved value")
    }
}

/// A node at protocol 15 holding the listings of both source contracts.
fn node_with_both_contracts() -> Node {
    let mut node = Node::new(protocol_15());
    for price in PRICES_A {
        node.list(CONTRACT_A, price);
    }
    for price in PRICES_B {
        node.list(CONTRACT_B, price);
    }
    node
}

/// The SUM of contract A's prices, past 64 bits, verifies whole from a signed answer.
#[test]
fn should_verify_a_signed_sum_past_64_bits() {
    let node = node_with_both_contracts();
    let DocumentSum(sum) = node.verify(node.query(SelectProjection::sum("price"), &[CONTRACT_A]));
    assert!(sum > i128::from(i64::MAX));
    assert_eq!(sum, total(&PRICES_A));
    assert_eq!(sum, 3 * i128::from(i64::MAX) - 1);
}

/// The AVG of contract A's prices verifies as the whole count and sum, and its exact
/// average keeps the digits an `f64` loses.
#[test]
fn should_verify_a_signed_average_past_64_bits() {
    let node = node_with_both_contracts();
    let average: DocumentAverage =
        node.verify(node.query(SelectProjection::avg("price"), &[CONTRACT_A]));
    assert_eq!(
        average,
        DocumentAverage {
            count: 3,
            sum: total(&PRICES_A),
        }
    );
    assert_eq!(
        average.to_decimal_string(4).as_deref(),
        Some("9223372036854775806.6667")
    );
}

/// A SUM grouped by the source contract, over an `in` on both contracts, verifies one
/// entry per contract keyed by its identifier's bytes: A's total past 64 bits, B's small.
#[test]
fn should_verify_signed_sums_grouped_by_contract_past_64_bits() {
    let node = node_with_both_contracts();
    let DocumentSplitSums(entries) =
        node.verify(node.query(SelectProjection::sum("price"), &[CONTRACT_A, CONTRACT_B]));
    assert_eq!(
        entries,
        vec![
            SplitSumEntry {
                in_key: Some(CONTRACT_A.to_vec()),
                key: Vec::new(),
                sum: Some(total(&PRICES_A)),
            },
            SplitSumEntry {
                in_key: Some(CONTRACT_B.to_vec()),
                key: Vec::new(),
                sum: Some(total(&PRICES_B)),
            },
        ]
    );
}

/// An AVG grouped by the source contract verifies each contract's whole count and sum, and
/// each entry's exact average.
#[test]
fn should_verify_signed_averages_grouped_by_contract_past_64_bits() {
    let node = node_with_both_contracts();
    let DocumentSplitAverages(entries) =
        node.verify(node.query(SelectProjection::avg("price"), &[CONTRACT_A, CONTRACT_B]));
    assert_eq!(
        entries,
        vec![
            SplitAverageEntry {
                in_key: Some(CONTRACT_A.to_vec()),
                key: Vec::new(),
                count: Some(3),
                sum: Some(total(&PRICES_A)),
            },
            SplitAverageEntry {
                in_key: Some(CONTRACT_B.to_vec()),
                key: Vec::new(),
                count: Some(2),
                sum: Some(total(&PRICES_B)),
            },
        ]
    );
    let averages: Vec<_> = entries
        .iter()
        .map(|entry| entry.to_decimal_string(4))
        .collect();
    assert_eq!(
        averages,
        vec![
            Some("9223372036854775806.6667".to_string()),
            Some("35002.5000".to_string()),
        ]
    );
}

/// Protocol 14 keeps listed prices in 64-bit trees, and a signed SUM and AVG over them
/// still verify through the same path.
#[test]
fn should_verify_a_signed_narrow_sum_and_average_at_protocol_14() {
    let mut node = Node::new(protocol_14());
    for price in PRICES_B {
        node.list(CONTRACT_B, price);
    }
    let DocumentSum(sum) = node.verify(node.query(SelectProjection::sum("price"), &[CONTRACT_B]));
    assert_eq!(sum, total(&PRICES_B));
    let average: DocumentAverage =
        node.verify(node.query(SelectProjection::avg("price"), &[CONTRACT_B]));
    assert_eq!(
        average,
        DocumentAverage {
            count: 2,
            sum: total(&PRICES_B),
        }
    );
    assert_eq!(average.to_decimal_string(4).as_deref(), Some("35002.5000"));
}
