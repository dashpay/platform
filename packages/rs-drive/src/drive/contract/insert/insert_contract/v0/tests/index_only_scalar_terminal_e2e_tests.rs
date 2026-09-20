//! End-to-end coverage for indexOnly **scalar terminals**: an index's
//! `terminal` may name any property a prefix position admits, not only
//! `$ownerId` or a refersTo identifier. The member key is the terminal
//! value in its tree-key encoding, so a 33-byte byte array, a string and
//! an integer key the `0` member bucket exactly as a prefix level would
//! key by them.
//!
//! Runs against the `index-only-scalar-terminal` fixture at
//! `tests/supporting_files/contract/index-only-scalar-terminal/`:
//!
//! | doctype  | index       | properties              | terminal   | terminal type  |
//! |----------|-------------|-------------------------|------------|----------------|
//! | `answer` | `byRequest` | `[requestId, $ownerId]` | `payload`  | 33-byte array  |
//! | `vote`   | `byPoll`    | `[pollId, $ownerId]`    | `choice`   | string ≤ 16    |
//! | `vote`   | `byChoice`  | `[pollId, choice]`      | `$ownerId` | ranked count   |
//! | `rating` | `byPost`    | `[postId, $ownerId]`    | `stars`    | integer 1 to 5 |
//!
//! Pinned: the entry layout (member key = encoded terminal value, element
//! = row-commitment `Item`), structural uniqueness spanning the terminal
//! value, query synthesis and proof parity with the terminal decoded off
//! the member key, terminal-equality lookups, delete-by-values symmetry,
//! and the fee invariant: the dry run's member-key width follows the
//! terminal property, so the estimate keeps upper-bounding the applied
//! fee for keys narrower and wider than 32 bytes.

use super::index_only_e2e_tests::{
    assert_grovedb_is_consistent, count_top_k, platform_version, read_grove_element,
};
use crate::drive::document::query::QueryDocumentsOutcomeV0Methods;
use crate::drive::document::INDEX_ONLY_ROW_COMMITMENT_SIZE;
use crate::drive::Drive;
use crate::error::Error;
use crate::query::{DriveDocumentQuery, InternalClauses, WhereClause, WhereOperator};
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::{Identifier, Value};
use dpp::prelude::DataContract;
use dpp::tests::json_document::json_document_to_contract;
use grovedb::Element;
use std::collections::BTreeMap;

const FIXTURE: &str = "tests/supporting_files/contract/index-only-scalar-terminal/index-only-scalar-terminal-contract.json";

const REQUEST: [u8; 20] = [0x5A; 20];
const PAYLOAD_1: [u8; 33] = [0x01; 33];
const PAYLOAD_2: [u8; 33] = [0x02; 33];
const POLL: [u8; 32] = [0xC3; 32];
const POST: [u8; 32] = [0xD4; 32];
const OWNER_1: [u8; 32] = [0x11; 32];
const OWNER_2: [u8; 32] = [0x22; 32];
const OWNER_3: [u8; 32] = [0x33; 32];

fn setup() -> (Drive, DataContract) {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = platform_version();
    let contract =
        json_document_to_contract(FIXTURE, false, pv).expect("expected to parse the fixture");
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("expected to apply the fixture contract");
    (drive, contract)
}

/// `[DataContractDocuments, contract_id, 1, <doctype>]`.
fn doctype_path(contract: &DataContract, doctype: &str) -> Vec<Vec<u8>> {
    vec![
        vec![crate::drive::RootTree::DataContractDocuments as u8],
        contract.id().as_bytes().to_vec(),
        vec![1],
        doctype.as_bytes().to_vec(),
    ]
}

fn build(
    contract: &DataContract,
    doctype: &str,
    properties: Vec<(&str, Value)>,
    owner: [u8; 32],
    seed: u64,
) -> Document {
    let document_type = contract
        .document_type_for_name(doctype)
        .expect("doctype exists");
    let mut document = document_type
        .random_document(Some(seed), platform_version())
        .expect("random document");
    document.set_properties(
        properties
            .into_iter()
            .map(|(name, value)| (name.to_string(), value))
            .collect::<BTreeMap<_, _>>(),
    );
    document.set_owner_id(Identifier::from(owner));
    document
}

fn build_answer(
    contract: &DataContract,
    request: [u8; 20],
    payload: [u8; 33],
    owner: [u8; 32],
    seed: u64,
) -> Document {
    build(
        contract,
        "answer",
        vec![
            ("requestId", Value::Bytes(request.to_vec())),
            ("payload", Value::Bytes(payload.to_vec())),
        ],
        owner,
        seed,
    )
}

fn build_vote(contract: &DataContract, choice: &str, owner: [u8; 32], seed: u64) -> Document {
    build(
        contract,
        "vote",
        vec![
            ("pollId", Value::Identifier(POLL)),
            ("choice", Value::Text(choice.to_string())),
        ],
        owner,
        seed,
    )
}

fn build_rating(contract: &DataContract, stars: u64, owner: [u8; 32], seed: u64) -> Document {
    build(
        contract,
        "rating",
        vec![
            ("postId", Value::Identifier(POST)),
            ("stars", Value::U64(stars)),
        ],
        owner,
        seed,
    )
}

fn insert(
    drive: &Drive,
    contract: &DataContract,
    doctype: &str,
    document: &Document,
    apply: bool,
) -> Result<FeeResult, Error> {
    let document_type = contract
        .document_type_for_name(doctype)
        .expect("doctype exists");
    drive.add_document_for_contract(
        DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((document, None)),
                owner_id: None,
            },
            contract,
            document_type,
        },
        false,
        BlockInfo::default(),
        apply,
        None,
        platform_version(),
        None,
    )
}

fn delete(
    drive: &Drive,
    contract: &DataContract,
    doctype: &str,
    document: Document,
    apply: bool,
) -> Result<FeeResult, Error> {
    let document_type = contract
        .document_type_for_name(doctype)
        .expect("doctype exists");
    drive.delete_index_only_document_for_contract(
        document,
        contract,
        document_type,
        BlockInfo::default(),
        apply,
        None,
        platform_version(),
        None,
    )
}

fn query<'a>(
    contract: &'a DataContract,
    doctype: &str,
    clauses: Vec<WhereClause>,
    limit: Option<u16>,
) -> DriveDocumentQuery<'a> {
    let document_type = contract
        .document_type_for_name(doctype)
        .expect("doctype exists");
    DriveDocumentQuery {
        contract,
        document_type,
        internal_clauses: InternalClauses::extract_from_clauses(clauses, platform_version())
            .expect("clauses extract"),
        offset: None,
        limit,
        order_by: Default::default(),
        start_at: None,
        start_at_included: false,
        block_time_ms: None,
        resolved_time_ranges: vec![],
        sub_queries: vec![],
    }
}

fn equal(field: &str, value: Value) -> WhereClause {
    WhereClause {
        field: field.to_string(),
        operator: WhereOperator::Equal,
        value,
    }
}

/// The entry under `[…prefix values, 0]` keyed by `member_key`.
fn entry(
    drive: &Drive,
    contract: &DataContract,
    doctype: &str,
    prefix: &[(&str, &[u8])],
    member_key: &[u8],
) -> Option<Element> {
    let mut path = doctype_path(contract, doctype);
    for (property, value) in prefix {
        path.push(property.as_bytes().to_vec());
        path.push(value.to_vec());
    }
    path.push(vec![0]);
    read_grove_element(drive, &path, member_key)
}

fn assert_commitment_item(element: Option<Element>, what: &str) {
    match element {
        Some(Element::Item(payload, _)) => assert_eq!(
            payload.len(),
            INDEX_ONLY_ROW_COMMITMENT_SIZE as usize,
            "{what}: the element is the row-commitment Item"
        ),
        other => panic!("{what}: expected a commitment Item, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Entry layout
// ---------------------------------------------------------------------------

/// Each doctype registers without a primary-key tree, and an insert keys
/// its member entry by the terminal's tree-key encoding: the raw 33 bytes
/// for the byte array, the UTF-8 bytes for the string, and the property's
/// integer key encoding — none of them 32 bytes wide.
#[test]
fn scalar_terminal_entries_are_keyed_by_the_encoded_terminal_value() {
    let (drive, contract) = setup();

    for doctype in ["answer", "vote", "rating"] {
        assert!(
            read_grove_element(&drive, &doctype_path(&contract, doctype), &[0]).is_none(),
            "{doctype}: an indexOnly doctype has no primary-key tree"
        );
    }

    let answer = build_answer(&contract, REQUEST, PAYLOAD_1, OWNER_1, 1);
    insert(&drive, &contract, "answer", &answer, true).expect("insert answer");
    assert_commitment_item(
        entry(
            &drive,
            &contract,
            "answer",
            &[("requestId", &REQUEST), ("$ownerId", &OWNER_1)],
            &PAYLOAD_1,
        ),
        "answer keyed by its 33-byte payload",
    );

    let vote = build_vote(&contract, "yes", OWNER_1, 2);
    insert(&drive, &contract, "vote", &vote, true).expect("insert vote");
    assert_commitment_item(
        entry(
            &drive,
            &contract,
            "vote",
            &[("pollId", &POLL), ("$ownerId", &OWNER_1)],
            b"yes",
        ),
        "vote keyed by its choice's UTF-8 bytes",
    );
    // The sibling index keys the same row the ordinary way round: the
    // choice in the prefix, the owner as the member key.
    assert_commitment_item(
        entry(
            &drive,
            &contract,
            "vote",
            &[("pollId", &POLL), ("choice", b"yes")],
            &OWNER_1,
        ),
        "vote under byChoice",
    );

    let rating = build_rating(&contract, 4, OWNER_1, 3);
    insert(&drive, &contract, "rating", &rating, true).expect("insert rating");
    let rating_type = contract
        .document_type_for_name("rating")
        .expect("rating doctype exists");
    let stars_key = rating_type
        .serialize_value_for_key("stars", &Value::U64(4), platform_version())
        .expect("the query-side encoding of the integer");
    assert_ne!(
        stars_key.len(),
        32,
        "an integer member key is not identifier-wide"
    );
    assert_commitment_item(
        entry(
            &drive,
            &contract,
            "rating",
            &[("postId", &POST), ("$ownerId", &OWNER_1)],
            &stars_key,
        ),
        "rating keyed by the query-side integer encoding",
    );

    assert_grovedb_is_consistent(&drive);
}

/// Structural uniqueness spans the terminal value: the same (request,
/// owner, payload) twice is a duplicate, while a second payload by the
/// same owner under the same request is a second entry.
#[test]
fn structural_uniqueness_spans_the_terminal_value() {
    let (drive, contract) = setup();
    let first = build_answer(&contract, REQUEST, PAYLOAD_1, OWNER_1, 1);
    insert(&drive, &contract, "answer", &first, true).expect("first insert");
    assert!(
        insert(&drive, &contract, "answer", &first, true).is_err(),
        "the same values twice is a duplicate entry"
    );
    let second = build_answer(&contract, REQUEST, PAYLOAD_2, OWNER_1, 2);
    insert(&drive, &contract, "answer", &second, true)
        .expect("a different terminal value by the same owner is a new entry");
    for payload in [PAYLOAD_1, PAYLOAD_2] {
        assert_commitment_item(
            entry(
                &drive,
                &contract,
                "answer",
                &[("requestId", &REQUEST), ("$ownerId", &OWNER_1)],
                &payload,
            ),
            "both payloads sit under the same prefix",
        );
    }
    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// Read surface
// ---------------------------------------------------------------------------

/// A prefix query synthesizes the terminal off the member key — the
/// 33-byte payload comes back as bytes, the choice as text — with proof
/// parity, and a terminal-equality clause answers "did this owner send
/// this payload" with an existence or absence proof.
#[test]
fn scalar_terminal_queries_synthesize_and_prove() {
    let (drive, contract) = setup();
    insert(
        &drive,
        &contract,
        "answer",
        &build_answer(&contract, REQUEST, PAYLOAD_1, OWNER_1, 1),
        true,
    )
    .expect("insert answer 1");
    insert(
        &drive,
        &contract,
        "answer",
        &build_answer(&contract, REQUEST, PAYLOAD_2, OWNER_2, 2),
        true,
    )
    .expect("insert answer 2");

    // ── every answer to the request ──
    let by_request = query(
        &contract,
        "answer",
        vec![equal("requestId", Value::Bytes(REQUEST.to_vec()))],
        Some(10),
    );
    let outcome = drive
        .query_documents(by_request.clone(), None, false, None, None)
        .expect("prefix query executes");
    let documents = outcome.documents();
    assert_eq!(documents.len(), 2, "two answers to the request");
    let mut seen: Vec<([u8; 32], Vec<u8>)> = documents
        .iter()
        .map(|document| {
            assert_eq!(
                document
                    .properties()
                    .get("requestId")
                    .expect("requestId recovered from the path")
                    .to_binary_bytes()
                    .expect("bytes"),
                REQUEST.to_vec()
            );
            let payload = document
                .properties()
                .get("payload")
                .expect("payload recovered from the member key")
                .to_binary_bytes()
                .expect("bytes");
            (document.owner_id().to_buffer(), payload)
        })
        .collect();
    seen.sort();
    assert_eq!(
        seen,
        vec![(OWNER_1, PAYLOAD_1.to_vec()), (OWNER_2, PAYLOAD_2.to_vec())],
        "synthesis must recover every (owner, payload) pair"
    );
    let (proof, _) = by_request
        .clone()
        .execute_with_proof(&drive, None, None, platform_version())
        .expect("proof generation");
    let (_root, verified) = by_request
        .verify_proof(proof.as_slice(), platform_version())
        .expect("proof verification synthesizes");
    let mut verified_ids: Vec<_> = verified.iter().map(|d| d.id()).collect();
    let mut queried_ids: Vec<_> = documents.iter().map(|d| d.id()).collect();
    verified_ids.sort();
    queried_ids.sort();
    assert_eq!(
        verified_ids, queried_ids,
        "proved and unproved synthesis agree"
    );
    let verified_payloads: Vec<Vec<u8>> = verified
        .iter()
        .map(|d| {
            d.properties()
                .get("payload")
                .expect("payload verified")
                .to_binary_bytes()
                .expect("bytes")
        })
        .collect();
    assert!(verified_payloads.contains(&PAYLOAD_1.to_vec()));
    assert!(verified_payloads.contains(&PAYLOAD_2.to_vec()));

    // ── terminal equality: did OWNER_1 send PAYLOAD_1 / PAYLOAD_2 ──
    let sent = query(
        &contract,
        "answer",
        vec![
            equal("requestId", Value::Bytes(REQUEST.to_vec())),
            equal("$ownerId", Value::Identifier(OWNER_1)),
            equal("payload", Value::Bytes(PAYLOAD_1.to_vec())),
        ],
        Some(1),
    );
    let outcome = drive
        .query_documents(sent.clone(), None, false, None, None)
        .expect("terminal-equality query executes");
    assert_eq!(outcome.documents().len(), 1);
    assert_eq!(outcome.documents()[0].owner_id().to_buffer(), OWNER_1);
    let (proof, _) = sent
        .clone()
        .execute_with_proof(&drive, None, None, platform_version())
        .expect("existence proof generation");
    let (_root, verified) = sent
        .verify_proof(proof.as_slice(), platform_version())
        .expect("existence proof verification");
    assert_eq!(verified.len(), 1);

    let not_sent = query(
        &contract,
        "answer",
        vec![
            equal("requestId", Value::Bytes(REQUEST.to_vec())),
            equal("$ownerId", Value::Identifier(OWNER_1)),
            equal("payload", Value::Bytes(PAYLOAD_2.to_vec())),
        ],
        Some(1),
    );
    let outcome = drive
        .query_documents(not_sent.clone(), None, false, None, None)
        .expect("negative terminal-equality query executes");
    assert!(
        outcome.documents().is_empty(),
        "OWNER_1 never sent PAYLOAD_2"
    );
    let (proof, _) = not_sent
        .clone()
        .execute_with_proof(&drive, None, None, platform_version())
        .expect("absence proof generation");
    let (_root, verified) = not_sent
        .verify_proof(proof.as_slice(), platform_version())
        .expect("absence proof verification");
    assert!(verified.is_empty(), "absence must verify as absence");

    assert_grovedb_is_consistent(&drive);
}

/// A string terminal: the poll's votes synthesize with their choices, a
/// terminal-equality clause on the string works, and the sibling ranked
/// index — which keys the same rows by owner — ranks the choices.
#[test]
fn string_terminal_votes_synthesize_and_rank() {
    let (drive, contract) = setup();
    for (choice, owner, seed) in [
        ("yes", OWNER_1, 1u64),
        ("yes", OWNER_2, 2),
        ("no", OWNER_3, 3),
    ] {
        insert(
            &drive,
            &contract,
            "vote",
            &build_vote(&contract, choice, owner, seed),
            true,
        )
        .expect("insert vote");
    }

    let poll_votes = query(
        &contract,
        "vote",
        vec![equal("pollId", Value::Identifier(POLL))],
        Some(10),
    );
    let outcome = drive
        .query_documents(poll_votes.clone(), None, false, None, None)
        .expect("poll query executes");
    let mut seen: Vec<([u8; 32], String)> = outcome
        .documents()
        .iter()
        .map(|document| {
            let choice = document
                .properties()
                .get("choice")
                .expect("choice recovered")
                .as_text()
                .expect("text")
                .to_string();
            (document.owner_id().to_buffer(), choice)
        })
        .collect();
    seen.sort();
    assert_eq!(
        seen,
        vec![
            (OWNER_1, "yes".to_string()),
            (OWNER_2, "yes".to_string()),
            (OWNER_3, "no".to_string()),
        ]
    );
    let (proof, _) = poll_votes
        .clone()
        .execute_with_proof(&drive, None, None, platform_version())
        .expect("proof generation");
    let (_root, verified) = poll_votes
        .verify_proof(proof.as_slice(), platform_version())
        .expect("proof verification");
    assert_eq!(verified.len(), 3);

    let owner_3_voted_no = query(
        &contract,
        "vote",
        vec![
            equal("pollId", Value::Identifier(POLL)),
            equal("$ownerId", Value::Identifier(OWNER_3)),
            equal("choice", Value::Text("no".to_string())),
        ],
        Some(1),
    );
    let outcome = drive
        .query_documents(owner_3_voted_no, None, false, None, None)
        .expect("string terminal equality executes");
    assert_eq!(outcome.documents().len(), 1);

    // byChoice ranks the choices by count under the poll.
    let mut choice_level = doctype_path(&contract, "vote");
    choice_level.extend([b"pollId".to_vec(), POLL.to_vec(), b"choice".to_vec()]);
    assert_eq!(
        count_top_k(&drive, &choice_level, 10, true),
        vec![(2, b"yes".to_vec()), (1, b"no".to_vec())],
        "the sibling ranked index composes with a string-terminal index on the same rows"
    );

    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// Delete symmetry and fees
// ---------------------------------------------------------------------------

/// Delete-by-values addresses the entry through its terminal value: a
/// delete carrying a different payload finds no entry and is refused, the
/// right one removes the entry, and a repeat is refused.
#[test]
fn scalar_terminal_delete_removes_the_entry_and_refuses_a_wrong_value() {
    let (drive, contract) = setup();
    let stored = build_answer(&contract, REQUEST, PAYLOAD_1, OWNER_1, 1);
    insert(&drive, &contract, "answer", &stored, true).expect("insert answer");
    let prefix: [(&str, &[u8]); 2] = [("requestId", &REQUEST), ("$ownerId", &OWNER_1)];

    let wrong = build_answer(&contract, REQUEST, PAYLOAD_2, OWNER_1, 2);
    assert!(
        delete(&drive, &contract, "answer", wrong, true).is_err(),
        "a delete naming a payload never written finds no entry"
    );
    assert_commitment_item(
        entry(&drive, &contract, "answer", &prefix, &PAYLOAD_1),
        "the stored entry survives the refused delete",
    );

    delete(&drive, &contract, "answer", stored.clone(), true).expect("delete by values");
    assert!(
        entry(&drive, &contract, "answer", &prefix, &PAYLOAD_1).is_none(),
        "the entry is gone"
    );
    assert!(
        delete(&drive, &contract, "answer", stored, true).is_err(),
        "deleting it again finds nothing"
    );
    assert_grovedb_is_consistent(&drive);
}

/// The dry run sizes the member key by the terminal property, so the
/// estimate upper-bounds the applied fee for a 33-byte key, a string key
/// and an integer key alike, on insert and on delete.
#[test]
fn scalar_terminal_estimated_fees_upper_bound_actual_fees() {
    let (drive, contract) = setup();
    let cases: Vec<(&str, Document)> = vec![
        (
            "answer",
            build_answer(&contract, REQUEST, PAYLOAD_1, OWNER_1, 1),
        ),
        ("vote", build_vote(&contract, "yes", OWNER_1, 2)),
        ("rating", build_rating(&contract, 4, OWNER_1, 3)),
    ];
    for (doctype, document) in cases {
        let estimated_insert =
            insert(&drive, &contract, doctype, &document, false).expect("estimated insert");
        let actual_insert =
            insert(&drive, &contract, doctype, &document, true).expect("actual insert");
        assert!(
            estimated_insert.storage_fee >= actual_insert.storage_fee,
            "{doctype}: estimated insert storage fee {} must upper-bound actual {}",
            estimated_insert.storage_fee,
            actual_insert.storage_fee
        );

        let estimated_delete =
            delete(&drive, &contract, doctype, document.clone(), false).expect("estimated delete");
        let actual_delete =
            delete(&drive, &contract, doctype, document, true).expect("actual delete");
        assert!(actual_delete.processing_fee > 0);
        assert!(
            estimated_delete.processing_fee >= actual_delete.processing_fee,
            "{doctype}: estimated delete processing fee {} must upper-bound actual {}",
            estimated_delete.processing_fee,
            actual_delete.processing_fee
        );
    }
    assert_grovedb_is_consistent(&drive);
}
