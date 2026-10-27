//! Every verifier that rebuilds a document query checks the answer only at the protocol
//! version that signed it.
//!
//! Each row calls one verifier with a response whose metadata names a protocol version. The
//! proof bytes are arbitrary: an answer the verifier refuses is refused before its proof is
//! read, and one it lets through fails on the proof bytes, so no case reaches the signature
//! check.
//!
//! These tests check drive-proof-verifier but live here, because building the queries needs
//! drive's server feature, which the client crate must not enable even as a dev-dependency.
//! The guard scan reads drive-proof-verifier's source.

use dapi_grpc::platform::v0::get_documents_response::{
    get_documents_response_v0, GetDocumentsResponseV0, Version,
};
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::{DocumentTypeRef, Index};
use dpp::data_contract::TokenConfiguration;
use dpp::platform_value::Value;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::tests::json_document::json_document_to_contract;
use dpp::version::PlatformVersion;
use drive::query::drive_document_having_query::mode_detection::detect_having_mode;
use drive::query::drive_document_having_query::resolve_having_query_for_mode;
use drive::query::drive_document_ranked_query::index_picker::resolve_ranked_query_for_mode;
use drive::query::drive_document_ranked_query::mode_detection::detect_ranked_mode;
use drive::query::drive_document_ranked_query::RankedPaginationInputs;
use drive::query::having::{
    HavingAggregate, HavingAggregateFunction, HavingClause, HavingOperator, HavingRightOperand,
};
use drive::query::projection::SelectProjection;
use drive::query::{
    DriveDocumentCountQuery, DriveDocumentQuery, DriveDocumentSumQuery, OrderClause,
};
use drive_proof_verifier::types::Documents;
use drive_proof_verifier::{
    verify_aggregate_count_and_sum_proof, verify_aggregate_count_proof, verify_aggregate_sum_proof,
    verify_carrier_aggregate_count_and_sum_proof, verify_carrier_aggregate_count_proof,
    verify_carrier_aggregate_sum_proof, verify_chained_documents_tenderdash_proof,
    verify_composite_documents_tenderdash_proof, verify_distinct_count_and_sum_proof,
    verify_distinct_count_proof, verify_distinct_sum_proof, verify_having_range_proof,
    verify_point_lookup_count_and_sum_proof, verify_point_lookup_count_proof,
    verify_point_lookup_sum_proof, verify_primary_key_count_sum_tree_proof,
    verify_primary_key_count_tree_proof, verify_primary_key_sum_tree_proof,
    verify_ranked_top_k_proof, ContextProvider, ContextProviderError, DocumentCount, Error,
    FromProof,
};
use std::sync::Arc;

/// Never consulted: no case reaches the signature check.
struct UnusedProvider;

impl ContextProvider for UnusedProvider {
    fn get_data_contract(
        &self,
        _: &Identifier,
        _: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        panic!("no case reads a contract")
    }
    fn get_token_configuration(
        &self,
        _: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        panic!("no case reads a token configuration")
    }
    fn get_quorum_public_key(
        &self,
        _: u32,
        _: [u8; 32],
        _: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        panic!("no case reaches the signature check")
    }
    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        panic!("no case reads the activation height")
    }
}

/// Each verifier that rebuilds a document query from a `GetDocumentsResponse` proof.
#[derive(Debug, Clone, Copy)]
enum Verifier {
    Documents,
    DocumentCount,
    AggregateCount,
    DistinctCount,
    PointLookupCount,
    PrimaryKeyCountTree,
    CarrierAggregateCount,
    AggregateSum,
    PrimaryKeySumTree,
    PointLookupSum,
    DistinctSum,
    CarrierAggregateSum,
    PointLookupCountAndSum,
    DistinctCountAndSum,
    AggregateCountAndSum,
    PrimaryKeyCountSumTree,
    CarrierAggregateCountAndSum,
    RankedTopK,
    HavingRange,
    CompositeDocuments,
    ChainedDocuments,
}

const VERIFIERS: [Verifier; 21] = [
    Verifier::Documents,
    Verifier::DocumentCount,
    Verifier::AggregateCount,
    Verifier::DistinctCount,
    Verifier::PointLookupCount,
    Verifier::PrimaryKeyCountTree,
    Verifier::CarrierAggregateCount,
    Verifier::AggregateSum,
    Verifier::PrimaryKeySumTree,
    Verifier::PointLookupSum,
    Verifier::DistinctSum,
    Verifier::CarrierAggregateSum,
    Verifier::PointLookupCountAndSum,
    Verifier::DistinctCountAndSum,
    Verifier::AggregateCountAndSum,
    Verifier::PrimaryKeyCountSumTree,
    Verifier::CarrierAggregateCountAndSum,
    Verifier::RankedTopK,
    Verifier::HavingRange,
    Verifier::CompositeDocuments,
    Verifier::ChainedDocuments,
];

/// The group every ranked and having query below groups by.
const GROUP_PROPERTY: &str = "restaurantId";

/// A contract with countable, summable and ranked indexes, so every query shape can be built.
fn restaurants() -> DataContract {
    json_document_to_contract(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../rs-drive/tests/supporting_files/contract/restaurants/restaurants-contract.json"
        ),
        false,
        PlatformVersion::latest(),
    )
    .expect("restaurants contract")
}

fn document_query<'a>(contract: &'a DataContract, document_type: &str) -> DriveDocumentQuery<'a> {
    DriveDocumentQuery::all_items_query(
        contract,
        contract.document_type_for_name(document_type).unwrap(),
        None,
    )
}

fn count_query<'a>(contract: &'a DataContract) -> DriveDocumentCountQuery<'a> {
    let document_type = contract.document_type_for_name("visit").unwrap();
    DriveDocumentCountQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: "visit".to_string(),
        index: first_index(contract, "visit"),
        where_clauses: vec![],
    }
}

fn sum_query<'a>(contract: &'a DataContract) -> DriveDocumentSumQuery<'a> {
    let document_type = contract.document_type_for_name("tip").unwrap();
    DriveDocumentSumQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: "tip".to_string(),
        index: first_index(contract, "tip"),
        where_clauses: vec![],
        sum_property: "amount".to_string(),
    }
}

fn first_index<'a>(contract: &'a DataContract, document_type: &str) -> &'a Index {
    contract
        .document_types()
        .get(document_type)
        .unwrap()
        .indexes()
        .values()
        .next()
        .unwrap()
}

fn document_type<'a>(contract: &'a DataContract, name: &str) -> DocumentTypeRef<'a> {
    contract.document_type_for_name(name).unwrap()
}

fn pagination() -> RankedPaginationInputs {
    RankedPaginationInputs {
        limit: Some(5),
        offset: None,
        has_start_at: false,
    }
}

fn response(mtd: &ResponseMetadata) -> GetDocumentsResponse {
    GetDocumentsResponse {
        version: Some(Version::V0(GetDocumentsResponseV0 {
            result: Some(get_documents_response_v0::Result::Proof(proof())),
            metadata: Some(mtd.clone()),
        })),
    }
}

fn proof() -> Proof {
    Proof {
        grovedb_proof: vec![1; 32],
        quorum_hash: vec![2; 32],
        signature: vec![3; 96],
        round: 1,
        block_id_hash: vec![4; 32],
        quorum_type: 106,
    }
}

/// Run `verifier` over an answer whose metadata says it was signed at `signed`, with the
/// rules of `platform_version`.
fn verify(
    verifier: Verifier,
    signed: u32,
    platform_version: &PlatformVersion,
) -> Result<(), Box<Error>> {
    let contract = restaurants();
    let contract_id = contract.id().to_buffer();
    let (proof, provider) = (&proof(), &UnusedProvider);
    let mtd = &ResponseMetadata {
        height: 10,
        time_ms: 1_700_000_000_000,
        protocol_version: signed,
        ..Default::default()
    };
    let pv = platform_version;
    let result = match verifier {
        Verifier::Documents => {
            <Documents as FromProof<DriveDocumentQuery>>::maybe_from_proof_with_metadata(
                document_query(&contract, "visit"),
                response(mtd),
                Network::Testnet,
                pv,
                provider,
            )
            .map(drop)
        }
        Verifier::DocumentCount => {
            <DocumentCount as FromProof<DriveDocumentQuery>>::maybe_from_proof_with_metadata(
                document_query(&contract, "visit"),
                response(mtd),
                Network::Testnet,
                pv,
                provider,
            )
            .map(drop)
        }
        Verifier::AggregateCount => {
            verify_aggregate_count_proof(&count_query(&contract), proof, mtd, pv, provider)
                .map(drop)
        }
        Verifier::DistinctCount => {
            verify_distinct_count_proof(&count_query(&contract), proof, mtd, 5, true, pv, provider)
                .map(drop)
        }
        Verifier::PointLookupCount => {
            verify_point_lookup_count_proof(&count_query(&contract), proof, mtd, pv, provider)
                .map(drop)
        }
        Verifier::PrimaryKeyCountTree => {
            verify_primary_key_count_tree_proof(contract_id, "visit", proof, mtd, pv, provider)
                .map(drop)
        }
        Verifier::CarrierAggregateCount => verify_carrier_aggregate_count_proof(
            &count_query(&contract),
            proof,
            mtd,
            Some(5),
            true,
            pv,
            provider,
        )
        .map(drop),
        Verifier::AggregateSum => {
            verify_aggregate_sum_proof(&sum_query(&contract), proof, mtd, pv, provider).map(drop)
        }
        Verifier::PrimaryKeySumTree => {
            verify_primary_key_sum_tree_proof(contract_id, "tip", proof, mtd, pv, provider)
                .map(drop)
        }
        Verifier::PointLookupSum => {
            verify_point_lookup_sum_proof(&sum_query(&contract), proof, mtd, pv, provider).map(drop)
        }
        Verifier::DistinctSum => {
            verify_distinct_sum_proof(&sum_query(&contract), proof, mtd, 5, true, pv, provider)
                .map(drop)
        }
        Verifier::CarrierAggregateSum => verify_carrier_aggregate_sum_proof(
            &sum_query(&contract),
            proof,
            mtd,
            Some(5),
            true,
            pv,
            provider,
        )
        .map(drop),
        Verifier::PointLookupCountAndSum => {
            verify_point_lookup_count_and_sum_proof(&sum_query(&contract), proof, mtd, pv, provider)
                .map(drop)
        }
        Verifier::DistinctCountAndSum => verify_distinct_count_and_sum_proof(
            &sum_query(&contract),
            proof,
            mtd,
            5,
            true,
            pv,
            provider,
        )
        .map(drop),
        Verifier::AggregateCountAndSum => {
            verify_aggregate_count_and_sum_proof(&sum_query(&contract), proof, mtd, pv, provider)
                .map(drop)
        }
        Verifier::PrimaryKeyCountSumTree => {
            verify_primary_key_count_sum_tree_proof(contract_id, "tip", proof, mtd, pv, provider)
                .map(drop)
        }
        Verifier::CarrierAggregateCountAndSum => verify_carrier_aggregate_count_and_sum_proof(
            &sum_query(&contract),
            proof,
            mtd,
            Some(5),
            true,
            pv,
            provider,
        )
        .map(drop),
        Verifier::RankedTopK => {
            let mode = detect_ranked_mode(
                &SelectProjection::avg("grade"),
                &[GROUP_PROPERTY.to_string()],
                &[],
                &[OrderClause {
                    field: "grade".to_string(),
                    ascending: false,
                }],
                &[],
                pagination(),
                PlatformVersion::latest(),
            )
            .expect("ranked shape");
            let query = resolve_ranked_query_for_mode(
                contract_id,
                document_type(&contract, "review"),
                "review".to_string(),
                contract.document_types().get("review").unwrap().indexes(),
                &mode,
                &[],
                PlatformVersion::latest(),
            )
            .expect("ranked index");
            verify_ranked_top_k_proof(&query, proof, mtd, pv, provider).map(drop)
        }
        Verifier::HavingRange => {
            let mode = detect_having_mode(
                &SelectProjection::count_star(),
                &[GROUP_PROPERTY.to_string()],
                &[HavingClause {
                    aggregate: HavingAggregate {
                        function: HavingAggregateFunction::Count,
                        field: String::new(),
                    },
                    operator: HavingOperator::GreaterThan,
                    right: HavingRightOperand::Value(Value::U64(1)),
                }],
                &[],
                &[],
                pagination(),
                PlatformVersion::latest(),
            )
            .expect("having shape");
            let query = resolve_having_query_for_mode(
                contract_id,
                document_type(&contract, "visit"),
                "visit".to_string(),
                contract.document_types().get("visit").unwrap().indexes(),
                &mode,
                &[],
                PlatformVersion::latest(),
            )
            .expect("having index");
            verify_having_range_proof(&query, proof, mtd, pv, provider).map(drop)
        }
        Verifier::CompositeDocuments => verify_composite_documents_tenderdash_proof(
            &document_query(&contract, "visit"),
            proof,
            mtd,
            pv,
            provider,
        )
        .map(drop),
        Verifier::ChainedDocuments => verify_chained_documents_tenderdash_proof(
            &document_query(&contract, "visit"),
            proof,
            mtd,
            pv,
            provider,
        )
        .map(drop),
    };
    result.map_err(Box::new)
}

fn latest() -> &'static PlatformVersion {
    PlatformVersion::latest()
}

fn previous() -> &'static PlatformVersion {
    PlatformVersion::get(latest().protocol_version - 1).expect("previous protocol version")
}

fn assert_refused(verifier: Verifier, signed: u32, platform_version: &PlatformVersion) {
    match verify(verifier, signed, platform_version).map_err(|error| *error) {
        Err(Error::ProofProtocolVersionMismatch {
            signed: refused,
            verifier: verified_with,
        }) => assert_eq!(
            (refused, verified_with),
            (signed, platform_version.protocol_version),
            "{verifier:?}"
        ),
        other => panic!(
            "{verifier:?}: an answer signed at {signed} verified with protocol {} must be \
             refused, got {other:?}",
            platform_version.protocol_version
        ),
    }
}

/// The answer is not refused: it reaches Drive's proof verification, which fails on the
/// arbitrary proof bytes or on the query, which is not one a node answers.
fn assert_reaches_proof_verification(
    verifier: Verifier,
    signed: u32,
    platform_version: &PlatformVersion,
) {
    match verify(verifier, signed, platform_version).map_err(|error| *error) {
        Err(Error::GroveDBError { .. } | Error::DriveError { .. }) => {}
        other => panic!(
            "{verifier:?}: an answer signed at {signed} verified with protocol {} must reach \
             proof verification and fail there, got {other:?}",
            platform_version.protocol_version
        ),
    }
}

#[test]
fn should_refuse_document_answers_signed_below_the_verifier_version() {
    let signed = previous().protocol_version;
    for verifier in VERIFIERS {
        assert_refused(verifier, signed, latest());
    }
}

#[test]
fn should_verify_document_answers_at_the_version_that_signed_them() {
    for verifier in VERIFIERS {
        assert_reaches_proof_verification(verifier, previous().protocol_version, previous());
        assert_reaches_proof_verification(verifier, latest().protocol_version, latest());
    }
}

/// A newer protocol than this build knows is verified with the newest rules it has.
#[test]
fn should_verify_document_answers_signed_by_an_unknown_newer_version_with_the_newest_rules() {
    let newer = latest().protocol_version + 1;
    for verifier in VERIFIERS {
        assert_reaches_proof_verification(verifier, newer, latest());
        assert_refused(verifier, newer, previous());
    }
}

#[test]
fn should_refuse_document_answers_signed_above_the_verifier_version() {
    let signed = latest().protocol_version;
    for verifier in VERIFIERS {
        assert_refused(verifier, signed, previous());
    }
}

#[test]
fn should_refuse_document_answers_without_a_signed_version() {
    for verifier in VERIFIERS {
        assert_refused(verifier, 0, latest());
    }
}

/// Reads the proof bytes of a document query answer only after its version is checked.
const CHECKED_PROOF_BYTES: &str = "document_query_proof_bytes(";
/// The crate's quorum signature checks.
const SIGNATURE_CHECKS: [&str; 2] = ["verify_tenderdash_proof(", "verify_tenderdash_signature("];
/// Every way to reach proof bytes without the version check names the proof field or a
/// helper named after it, such as `supported_grovedb_proof_bytes`.
const UNCHECKED_PROOF_BYTES: &str = "grovedb_proof";

/// Whether a line opens a top-level function or impl, whatever its visibility and qualifiers.
fn opens_item(line: &str) -> bool {
    if line.starts_with(char::is_whitespace) {
        return false;
    }
    line.split_whitespace()
        .find(|word| !(word.starts_with("pub") || ["async", "const", "unsafe"].contains(word)))
        .is_some_and(|word| word == "fn" || word.starts_with("impl"))
}

/// The top-level items of a source file before its tests, without comments: each starts at a
/// function or impl and runs to the next one. Test code starts at the first top-level
/// `#[cfg(...)]` naming `test`; verifier modules keep their tests at the end.
fn items(source: &str) -> Vec<String> {
    let mut items: Vec<String> = Vec::new();
    for line in source.lines() {
        if line.starts_with("#[cfg(") && line.contains("test") {
            break;
        }
        if opens_item(line) || items.is_empty() {
            items.push(String::new());
        }
        if !line.trim_start().starts_with("//") {
            let item = items.last_mut().unwrap();
            item.push_str(line);
            item.push('\n');
        }
    }
    items
}

/// Checks one verifier: it reads proof bytes only through [`CHECKED_PROOF_BYTES`], once per
/// signature check and before it. Returns how many signature checks it makes.
fn assert_checks_version_before_reading_the_proof(item: &str, at: &str) -> usize {
    assert!(
        !item.contains(UNCHECKED_PROOF_BYTES),
        "{at} reads proof bytes without the version check"
    );
    let mut signature_checks: Vec<usize> = SIGNATURE_CHECKS
        .iter()
        .flat_map(|check| item.match_indices(check).map(|(at, _)| at))
        .collect();
    signature_checks.sort();
    assert_eq!(
        item.matches(CHECKED_PROOF_BYTES).count(),
        signature_checks.len(),
        "{at}"
    );
    for (nth, signature_check) in signature_checks.iter().enumerate() {
        assert!(
            item[..*signature_check]
                .matches(CHECKED_PROOF_BYTES)
                .count()
                > nth,
            "{at} checks a signature before reading its proof through the version check"
        );
    }
    signature_checks.len()
}

/// Every document query verifier reads its proof only through the accessor that refuses an
/// answer at another protocol version, and checks the signature only after that. A new
/// document verifier written otherwise fails here, however it is formatted:
/// - a verifier module that uses document query types reads no proof bytes another way, and
///   each of its functions or impls that checks a signature reads the proof through the
///   accessor first;
/// - in `proof.rs`, where most verifiers are not for documents, each `GetDocumentsResponse`
///   impl does the same.
#[test]
fn should_read_every_document_query_proof_after_checking_its_version() {
    let source_dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../rs-drive-proof-verifier/src"
    );
    let mut checked = 0;
    for entry in std::fs::read_dir(format!("{source_dir}/proof")).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        let source = std::fs::read_to_string(&path).unwrap();
        if !(source.contains("DriveDocument") || source.contains("GetDocumentsResponse")) {
            continue;
        }
        for (index, item) in items(&source).iter().enumerate() {
            checked +=
                assert_checks_version_before_reading_the_proof(item, &format!("{name} #{index}"));
        }
    }
    let proof = std::fs::read_to_string(format!("{source_dir}/proof.rs")).unwrap();
    let document_impls: Vec<String> = items(&proof)
        .into_iter()
        .filter(|item| {
            item.lines()
                .any(|line| line.contains("type Response") && line.contains("GetDocumentsResponse"))
        })
        .collect();
    for (index, item) in document_impls.iter().enumerate() {
        let checks = assert_checks_version_before_reading_the_proof(
            item,
            &format!("proof.rs impl #{index}"),
        );
        assert!(checks > 0, "proof.rs impl #{index} checks no signature");
        checked += checks;
    }
    assert_eq!(
        proof.matches(CHECKED_PROOF_BYTES).count(),
        document_impls
            .iter()
            .map(|item| item.matches(CHECKED_PROOF_BYTES).count())
            .sum::<usize>(),
        "proof.rs reads checked document proof bytes outside its document impls"
    );
    assert_eq!(checked, VERIFIERS.len());
}
