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
//! | `reaction` | `byPostKind` | `[postId]`          | `kind ‖ $ownerId` | composite, prefixed |
//! | `loginKeyResponse` | `byRequest` | (none: flat) | `appEphemeralPubKeyHash ‖ $ownerId` | composite, flat; `entryPayload` |
//! | `note`   | `byPostBody` | `[postId]`            | `$ownerId ‖ body` | composite; variable-width last |
//!
//! The last two exercise **composite terminals** (the member key is the
//! concatenation of several components' encodings) — one below a prefix
//! level, one FLAT (no prefix at all, entries directly under a level keyed
//! by the terminal's names) — and the flat one also carries an
//! **entry payload**: the wallet key and the ciphertext ride in every
//! entry's item after the row commitment, the type's value slot.
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
use crate::query::{DriveDocumentQuery, InternalClauses, OrderClause, WhereClause, WhereOperator};
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::{Identifier, Value};
use dpp::prelude::DataContract;
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
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

/// A fresh Drive with the scalar-terminal fixture contract applied.
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

/// A random document of `doctype` with the given properties and owner set.
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

/// An `answer` document: a request hash with its response payload.
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

/// A `vote` document for `choice` by `owner`.
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

/// A `rating` document of `stars` by `owner`.
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

/// Inserts (or dry-runs, when `apply` is false) the document and returns its fee.
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

/// Deletes (or dry-runs, when `apply` is false) the document and returns its fee.
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

/// A query on `doctype` with the given clauses and no ordering.
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

/// An equality clause on `field`.
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

/// Asserts the element is a bare row-commitment Item (no entry payload).
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

// ---------------------------------------------------------------------------
// Composite terminals and the entry payload
// ---------------------------------------------------------------------------

const REQUEST_HASH: [u8; 20] = [0x7A; 20];
const OTHER_REQUEST_HASH: [u8; 20] = [0x7B; 20];
const WALLET_KEY_1: [u8; 33] = [0xA1; 33];
const WALLET_KEY_2: [u8; 33] = [0xA2; 33];
const WALLET_KEY_3: [u8; 33] = [0xA3; 33];
/// The flat level of `loginKeyResponse.byRequest`: a zero byte, then each
/// terminal component name preceded by a zero byte.
const LOGIN_FLAT_LEVEL: &[u8] = b"\0appEphemeralPubKeyHash\0$ownerId";

/// A stand-in ciphertext of `len` copies of `byte`.
fn cipher(byte: u8, len: usize) -> Vec<u8> {
    vec![byte; len]
}

/// A `loginKeyResponse` document for `request` with the wallet key and ciphertext payload.
fn build_login_response(
    contract: &DataContract,
    request: [u8; 20],
    wallet_key: [u8; 33],
    ciphertext: Vec<u8>,
    owner: [u8; 32],
    seed: u64,
) -> Document {
    build(
        contract,
        "loginKeyResponse",
        vec![
            ("appEphemeralPubKeyHash", Value::Bytes(request.to_vec())),
            ("walletEphemeralPubKey", Value::Bytes(wallet_key.to_vec())),
            ("encryptedPayload", Value::Bytes(ciphertext)),
        ],
        owner,
        seed,
    )
}

/// A `reaction` document of `kind` on the fixture post by `owner`.
fn build_reaction(contract: &DataContract, kind: u64, owner: [u8; 32], seed: u64) -> Document {
    build(
        contract,
        "reaction",
        vec![
            ("postId", Value::Identifier(POST)),
            ("kind", Value::U64(kind)),
        ],
        owner,
        seed,
    )
}

/// A query on `doctype` with the given clauses and `order_by` (field, ascending).
fn query_ordered<'a>(
    contract: &'a DataContract,
    doctype: &str,
    clauses: Vec<WhereClause>,
    order_by: Vec<(&str, bool)>,
    limit: Option<u16>,
) -> DriveDocumentQuery<'a> {
    let mut query = query(contract, doctype, clauses, limit);
    query.order_by = order_by
        .into_iter()
        .map(|(field, ascending)| {
            (
                field.to_string(),
                OrderClause {
                    field: field.to_string(),
                    ascending,
                },
            )
        })
        .collect();
    query
}

/// The flat member key of a login response: request hash then owner id.
fn login_member_key(request: [u8; 20], owner: [u8; 32]) -> Vec<u8> {
    let mut key = request.to_vec();
    key.extend(owner);
    key
}

/// Reads the login response entry stored under the flat level for `request` and `owner`.
fn login_entry(
    drive: &Drive,
    contract: &DataContract,
    request: [u8; 20],
    owner: [u8; 32],
) -> Option<Element> {
    let mut path = doctype_path(contract, "loginKeyResponse");
    path.push(LOGIN_FLAT_LEVEL.to_vec());
    path.push(vec![0]);
    read_grove_element(drive, &path, &login_member_key(request, owner))
}

/// The binary bytes of the document property `name`.
fn payload_bytes(document: &Document, name: &str) -> Vec<u8> {
    document
        .properties()
        .get(name)
        .unwrap_or_else(|| panic!("{name} present"))
        .to_binary_bytes()
        .expect("bytes")
}

/// A flat composite index writes its entry directly under its own level:
/// `[…, "\0appEphemeralPubKeyHash\0$ownerId", 0, hash ‖ owner]`, no
/// property-name tree and no value tree for either component. The item is
/// the row commitment followed by the entry payload — each payload property
/// in name order, length-framed: the ciphertext first, then the wallet key.
#[test]
fn flat_composite_entries_carry_the_payload_in_the_item() {
    let (drive, contract) = setup();
    let ciphertext = cipher(0xC1, 60);
    let response = build_login_response(
        &contract,
        REQUEST_HASH,
        WALLET_KEY_1,
        ciphertext.clone(),
        OWNER_1,
        1,
    );
    insert(&drive, &contract, "loginKeyResponse", &response, true).expect("insert response");

    match login_entry(&drive, &contract, REQUEST_HASH, OWNER_1) {
        Some(Element::Item(bytes, _)) => {
            let commitment_size = INDEX_ONLY_ROW_COMMITMENT_SIZE as usize;
            assert_eq!(bytes.len(), commitment_size + 2 + 60 + 2 + 33);
            let mut cursor = commitment_size;
            assert_eq!(&bytes[cursor..cursor + 2], &60u16.to_be_bytes());
            cursor += 2;
            assert_eq!(&bytes[cursor..cursor + 60], ciphertext.as_slice());
            cursor += 60;
            assert_eq!(&bytes[cursor..cursor + 2], &33u16.to_be_bytes());
            cursor += 2;
            assert_eq!(&bytes[cursor..], &WALLET_KEY_1);
        }
        other => panic!("expected the payload-bearing Item, got {other:?}"),
    }
    let base = doctype_path(&contract, "loginKeyResponse");
    assert!(
        read_grove_element(&drive, &base, b"appEphemeralPubKeyHash").is_none()
            && read_grove_element(&drive, &base, b"$ownerId").is_none(),
        "a flat index owns no property-name trees"
    );
    assert!(
        read_grove_element(&drive, &base, LOGIN_FLAT_LEVEL).is_some(),
        "the flat level tree sits directly under the doctype"
    );

    // The same values by the same owner are a duplicate; another owner is
    // a second entry under the same request hash.
    assert!(insert(&drive, &contract, "loginKeyResponse", &response, true).is_err());
    insert(
        &drive,
        &contract,
        "loginKeyResponse",
        &build_login_response(
            &contract,
            REQUEST_HASH,
            WALLET_KEY_2,
            cipher(0xC2, 92),
            OWNER_2,
            2,
        ),
        true,
    )
    .expect("a second responder");
    assert!(login_entry(&drive, &contract, REQUEST_HASH, OWNER_2).is_some());

    assert_grovedb_is_consistent(&drive);
}

/// The app's read: equality on the request hash (the leading component)
/// returns every responder's owner id with the wallet key and ciphertext
/// decoded off the item, as one proof; a point lookup on hash and owner
/// answers with existence or absence; a clause-free query scans the flat
/// level.
#[test]
fn flat_composite_lookup_by_request_hash_synthesizes_the_payload_and_proves() {
    let (drive, contract) = setup();
    let cipher_1 = cipher(0xC1, 60);
    let cipher_2 = cipher(0xC2, 572);
    for (request, key, ciphertext, owner, seed) in [
        (REQUEST_HASH, WALLET_KEY_1, cipher_1.clone(), OWNER_1, 1u64),
        (REQUEST_HASH, WALLET_KEY_2, cipher_2.clone(), OWNER_2, 2),
        (
            OTHER_REQUEST_HASH,
            WALLET_KEY_3,
            cipher(0xC3, 100),
            OWNER_3,
            3,
        ),
    ] {
        insert(
            &drive,
            &contract,
            "loginKeyResponse",
            &build_login_response(&contract, request, key, ciphertext, owner, seed),
            true,
        )
        .expect("insert response");
    }

    // ── every answer to the request ──
    let by_request = query(
        &contract,
        "loginKeyResponse",
        vec![equal(
            "appEphemeralPubKeyHash",
            Value::Bytes(REQUEST_HASH.to_vec()),
        )],
        Some(10),
    );
    let outcome = drive
        .query_documents(by_request.clone(), None, false, None, None)
        .expect("lookup by request hash executes");
    let documents = outcome.documents();
    assert_eq!(documents.len(), 2, "two responders to the request");
    let mut seen: Vec<([u8; 32], Vec<u8>, Vec<u8>)> = documents
        .iter()
        .map(|document| {
            assert_eq!(
                payload_bytes(document, "appEphemeralPubKeyHash"),
                REQUEST_HASH.to_vec()
            );
            (
                document.owner_id().to_buffer(),
                payload_bytes(document, "walletEphemeralPubKey"),
                payload_bytes(document, "encryptedPayload"),
            )
        })
        .collect();
    seen.sort();
    assert_eq!(
        seen,
        vec![
            (OWNER_1, WALLET_KEY_1.to_vec(), cipher_1.clone()),
            (OWNER_2, WALLET_KEY_2.to_vec(), cipher_2.clone()),
        ],
        "the owner comes off the member key, the wallet key and ciphertext off the item"
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
    let verified_ciphers: Vec<Vec<u8>> = verified
        .iter()
        .map(|d| payload_bytes(d, "encryptedPayload"))
        .collect();
    assert!(verified_ciphers.contains(&cipher_1) && verified_ciphers.contains(&cipher_2));

    // ── point lookup: did OWNER_1 / OWNER_3 answer this request ──
    let answered = query(
        &contract,
        "loginKeyResponse",
        vec![
            equal(
                "appEphemeralPubKeyHash",
                Value::Bytes(REQUEST_HASH.to_vec()),
            ),
            equal("$ownerId", Value::Identifier(OWNER_1)),
        ],
        Some(1),
    );
    let outcome = drive
        .query_documents(answered.clone(), None, false, None, None)
        .expect("point lookup executes");
    assert_eq!(outcome.documents().len(), 1);
    assert_eq!(
        payload_bytes(&outcome.documents()[0], "walletEphemeralPubKey"),
        WALLET_KEY_1.to_vec()
    );
    let (proof, _) = answered
        .clone()
        .execute_with_proof(&drive, None, None, platform_version())
        .expect("existence proof generation");
    let (_root, verified) = answered
        .verify_proof(proof.as_slice(), platform_version())
        .expect("existence proof verification");
    assert_eq!(verified.len(), 1);

    let not_answered = query(
        &contract,
        "loginKeyResponse",
        vec![
            equal(
                "appEphemeralPubKeyHash",
                Value::Bytes(REQUEST_HASH.to_vec()),
            ),
            equal("$ownerId", Value::Identifier(OWNER_3)),
        ],
        Some(1),
    );
    assert!(drive
        .query_documents(not_answered.clone(), None, false, None, None)
        .expect("negative point lookup executes")
        .documents()
        .is_empty());
    let (proof, _) = not_answered
        .clone()
        .execute_with_proof(&drive, None, None, platform_version())
        .expect("absence proof generation");
    let (_root, verified) = not_answered
        .verify_proof(proof.as_slice(), platform_version())
        .expect("absence proof verification");
    assert!(verified.is_empty(), "absence must verify as absence");

    // ── everything: a clause-free query scans the flat level ──
    let everything = query(&contract, "loginKeyResponse", vec![], Some(10));
    let outcome = drive
        .query_documents(everything.clone(), None, false, None, None)
        .expect("flat scan executes");
    assert_eq!(outcome.documents().len(), 3);
    let (proof, _) = everything
        .clone()
        .execute_with_proof(&drive, None, None, platform_version())
        .expect("flat scan proof generation");
    let (_root, verified) = everything
        .verify_proof(proof.as_slice(), platform_version())
        .expect("flat scan proof verification");
    assert_eq!(verified.len(), 3);

    assert_grovedb_is_consistent(&drive);
}

/// Keyset pagination over the second component: with the request hash
/// bound, `$ownerId > <last seen>` ordered by `$ownerId` walks the
/// responders page by page, each page agreeing with its proof.
#[test]
fn flat_composite_keyset_pagination_over_the_second_component() {
    let (drive, contract) = setup();
    for (key, owner, seed) in [
        (WALLET_KEY_1, OWNER_1, 1u64),
        (WALLET_KEY_2, OWNER_2, 2),
        (WALLET_KEY_3, OWNER_3, 3),
    ] {
        insert(
            &drive,
            &contract,
            "loginKeyResponse",
            &build_login_response(
                &contract,
                REQUEST_HASH,
                key,
                cipher(seed as u8, 60),
                owner,
                seed,
            ),
            true,
        )
        .expect("insert response");
    }

    let page_1 = query_ordered(
        &contract,
        "loginKeyResponse",
        vec![equal(
            "appEphemeralPubKeyHash",
            Value::Bytes(REQUEST_HASH.to_vec()),
        )],
        vec![("$ownerId", true)],
        Some(2),
    );
    let outcome = drive
        .query_documents(page_1.clone(), None, false, None, None)
        .expect("page 1 executes");
    let owners: Vec<[u8; 32]> = outcome
        .documents()
        .iter()
        .map(|d| d.owner_id().to_buffer())
        .collect();
    assert_eq!(owners, vec![OWNER_1, OWNER_2]);
    let (proof, _) = page_1
        .clone()
        .execute_with_proof(&drive, None, None, platform_version())
        .expect("page 1 proof");
    let (_root, verified) = page_1
        .verify_proof(proof.as_slice(), platform_version())
        .expect("page 1 verifies");
    assert_eq!(verified.len(), 2);

    let page_2 = query_ordered(
        &contract,
        "loginKeyResponse",
        vec![
            equal(
                "appEphemeralPubKeyHash",
                Value::Bytes(REQUEST_HASH.to_vec()),
            ),
            WhereClause {
                field: "$ownerId".to_string(),
                operator: WhereOperator::GreaterThan,
                value: Value::Identifier(OWNER_2),
            },
        ],
        vec![("$ownerId", true)],
        Some(2),
    );
    let outcome = drive
        .query_documents(page_2.clone(), None, false, None, None)
        .expect("page 2 executes");
    let owners: Vec<[u8; 32]> = outcome
        .documents()
        .iter()
        .map(|d| d.owner_id().to_buffer())
        .collect();
    assert_eq!(owners, vec![OWNER_3]);
    assert_eq!(
        payload_bytes(&outcome.documents()[0], "walletEphemeralPubKey"),
        WALLET_KEY_3.to_vec()
    );
    let (proof, _) = page_2
        .clone()
        .execute_with_proof(&drive, None, None, platform_version())
        .expect("page 2 proof");
    let (_root, verified) = page_2
        .verify_proof(proof.as_slice(), platform_version())
        .expect("page 2 verifies");
    assert_eq!(verified.len(), 1);

    assert_grovedb_is_consistent(&drive);
}

/// Delete-by-values on a flat composite entry with a payload: a delete
/// carrying a different ciphertext recomputes a different commitment and
/// is refused, the right one removes the entry, and the dry run
/// upper-bounds the applied fee on insert and delete (the item is sized by
/// the payload bound, the key by the components' widths).
#[test]
fn flat_composite_delete_and_fees() {
    let (drive, contract) = setup();
    let stored = build_login_response(
        &contract,
        REQUEST_HASH,
        WALLET_KEY_1,
        cipher(0xC1, 572),
        OWNER_1,
        1,
    );
    let estimated_insert =
        insert(&drive, &contract, "loginKeyResponse", &stored, false).expect("estimated insert");
    let actual_insert =
        insert(&drive, &contract, "loginKeyResponse", &stored, true).expect("actual insert");
    assert!(
        estimated_insert.storage_fee >= actual_insert.storage_fee,
        "estimated insert storage fee {} must upper-bound actual {}",
        estimated_insert.storage_fee,
        actual_insert.storage_fee
    );

    let wrong = build_login_response(
        &contract,
        REQUEST_HASH,
        WALLET_KEY_1,
        cipher(0xC9, 572),
        OWNER_1,
        2,
    );
    assert!(
        delete(&drive, &contract, "loginKeyResponse", wrong, true).is_err(),
        "a delete whose payload disagrees with the stored entry fails the commitment probe"
    );
    assert!(login_entry(&drive, &contract, REQUEST_HASH, OWNER_1).is_some());

    let estimated_delete = delete(&drive, &contract, "loginKeyResponse", stored.clone(), false)
        .expect("estimated delete");
    let actual_delete =
        delete(&drive, &contract, "loginKeyResponse", stored, true).expect("actual delete");
    assert!(actual_delete.processing_fee > 0);
    assert!(
        estimated_delete.processing_fee >= actual_delete.processing_fee,
        "estimated delete processing fee {} must upper-bound actual {}",
        estimated_delete.processing_fee,
        actual_delete.processing_fee
    );
    assert!(login_entry(&drive, &contract, REQUEST_HASH, OWNER_1).is_none());
    // The flat level survives the last entry's removal: it is registration
    // structure, and the next insert lands under it again.
    assert!(read_grove_element(
        &drive,
        &doctype_path(&contract, "loginKeyResponse"),
        LOGIN_FLAT_LEVEL
    )
    .is_some());
    insert(
        &drive,
        &contract,
        "loginKeyResponse",
        &build_login_response(
            &contract,
            REQUEST_HASH,
            WALLET_KEY_2,
            cipher(0xC2, 60),
            OWNER_2,
            3,
        ),
        true,
    )
    .expect("insert after the level was drained");

    assert_grovedb_is_consistent(&drive);
}

/// A composite terminal BELOW a prefix level: `[postId] → kind ‖ $ownerId`.
/// The entry sits in the post's `0` bucket keyed by the integer's tree-key
/// encoding followed by the owner; equality on `kind` (the first
/// component) with the post bound lowers onto a key range and returns the
/// reacting owners; the hierarchical machinery above is untouched.
#[test]
fn prefixed_composite_terminal_ranges_over_the_leading_component() {
    let (drive, contract) = setup();
    for (kind, owner, seed) in [(3u64, OWNER_1, 1u64), (3, OWNER_2, 2), (7, OWNER_3, 3)] {
        insert(
            &drive,
            &contract,
            "reaction",
            &build_reaction(&contract, kind, owner, seed),
            true,
        )
        .expect("insert reaction");
    }
    let reaction_type = contract
        .document_type_for_name("reaction")
        .expect("reaction doctype exists");
    let mut expected_key = reaction_type
        .serialize_value_for_key("kind", &Value::U64(3), platform_version())
        .expect("kind encodes");
    expected_key.extend(OWNER_1);
    assert_commitment_item(
        entry(
            &drive,
            &contract,
            "reaction",
            &[("postId", &POST)],
            &expected_key,
        ),
        "reaction keyed by kind ‖ owner under the post",
    );

    let thumbs = query(
        &contract,
        "reaction",
        vec![
            equal("postId", Value::Identifier(POST)),
            equal("kind", Value::U64(3)),
        ],
        Some(10),
    );
    let outcome = drive
        .query_documents(thumbs.clone(), None, false, None, None)
        .expect("leading-component equality executes");
    let mut owners: Vec<[u8; 32]> = outcome
        .documents()
        .iter()
        .map(|d| d.owner_id().to_buffer())
        .collect();
    owners.sort();
    assert_eq!(owners, vec![OWNER_1, OWNER_2]);
    for document in outcome.documents() {
        assert_eq!(
            document
                .properties()
                .get("kind")
                .and_then(|v| v.to_integer::<u64>().ok()),
            Some(3),
            "the leading component is decoded off the member key"
        );
    }
    let (proof, _) = thumbs
        .clone()
        .execute_with_proof(&drive, None, None, platform_version())
        .expect("proof generation");
    let (_root, verified) = thumbs
        .verify_proof(proof.as_slice(), platform_version())
        .expect("proof verification");
    assert_eq!(verified.len(), 2);

    let all_reactions = query(
        &contract,
        "reaction",
        vec![equal("postId", Value::Identifier(POST))],
        Some(10),
    );
    let outcome = drive
        .query_documents(all_reactions, None, false, None, None)
        .expect("prefix query executes");
    assert_eq!(outcome.documents().len(), 3);

    assert_grovedb_is_consistent(&drive);
}

#[test]
fn should_reject_unrepresentable_composite_terminal_ordering() {
    let (drive, contract) = setup();
    for (kind, owner, seed) in [(3, OWNER_1, 1), (3, OWNER_3, 2), (7, OWNER_2, 3)] {
        insert(
            &drive,
            &contract,
            "reaction",
            &build_reaction(&contract, kind, owner, seed),
            true,
        )
        .expect("insert reaction");
    }
    let (valid_proof, _) = query_ordered(
        &contract,
        "reaction",
        vec![equal("postId", Value::Identifier(POST))],
        vec![("kind", true), ("$ownerId", true)],
        Some(10),
    )
    .execute_with_proof(&drive, None, None, platform_version())
    .expect("prove the actual member-key order");
    for order_by in [
        vec![("$ownerId", true)],
        vec![("$ownerId", true), ("kind", true)],
        vec![("kind", true), ("$ownerId", false)],
    ] {
        let query = query_ordered(
            &contract,
            "reaction",
            vec![equal("postId", Value::Identifier(POST))],
            order_by,
            Some(10),
        );
        // A run of components out of declared order is refused by the
        // matcher itself (no index can serve it: "valid indexes are");
        // the other shapes reach the terminal route and fail its orderBy
        // rule. Either way the executor, the prover and the verifier
        // refuse identically.
        let unrepresentable = |error: &Error| {
            error.to_string().contains("orderBy") || error.to_string().contains("valid indexes are")
        };
        let error = drive
            .query_documents(query.clone(), None, false, None, None)
            .expect_err("one member-key walk cannot implement this ordering");
        assert!(unrepresentable(&error), "{error}");
        let error = query
            .clone()
            .execute_with_proof(&drive, None, None, platform_version())
            .expect_err("the prover must reject the same ordering");
        assert!(unrepresentable(&error), "{error}");
        let error = query
            .verify_proof(&valid_proof, platform_version())
            .expect_err("a proof of member-key order cannot prove a different sort order");
        assert!(unrepresentable(&error), "{error}");
    }
}

#[test]
fn should_order_composite_terminal_components_in_both_directions() {
    let (drive, contract) = setup();
    for (kind, owner, seed) in [(3, OWNER_1, 1), (3, OWNER_3, 2), (7, OWNER_2, 3)] {
        insert(
            &drive,
            &contract,
            "reaction",
            &build_reaction(&contract, kind, owner, seed),
            true,
        )
        .expect("insert reaction");
    }
    for (order_by, kind, expected) in [
        (
            vec![("kind", true), ("$ownerId", true)],
            None,
            vec![OWNER_1, OWNER_3, OWNER_2],
        ),
        (
            vec![("kind", false), ("$ownerId", false)],
            None,
            vec![OWNER_2, OWNER_3, OWNER_1],
        ),
        // Equality-bound components do not affect ordering, so their
        // direction need not agree with the remaining member-key order.
        (
            vec![("kind", true), ("$ownerId", false)],
            Some(3),
            vec![OWNER_3, OWNER_1],
        ),
        (vec![("$ownerId", true)], Some(3), vec![OWNER_1, OWNER_3]),
    ] {
        let mut clauses = vec![equal("postId", Value::Identifier(POST))];
        if let Some(kind) = kind {
            clauses.push(equal("kind", Value::U64(kind)));
        }
        let query = query_ordered(&contract, "reaction", clauses, order_by, Some(10));
        let outcome = drive
            .query_documents(query.clone(), None, false, None, None)
            .expect("representable ordering executes");
        let owners: Vec<_> = outcome
            .documents()
            .iter()
            .map(|d| d.owner_id().to_buffer())
            .collect();
        assert_eq!(owners, expected);
        let (proof, _) = query
            .clone()
            .execute_with_proof(&drive, None, None, platform_version())
            .expect("prove representable ordering");
        let (_, verified) = query
            .verify_proof(&proof, platform_version())
            .expect("verify ordering");
        let owners: Vec<_> = verified.iter().map(|d| d.owner_id().to_buffer()).collect();
        assert_eq!(owners, expected);
    }
}

#[test]
fn should_roundtrip_empty_and_nul_entry_payloads_and_bind_deletes() {
    let (drive, contract) = setup();
    // Full contract validation admits both zero-length payloads and a NUL
    // string. Neither needs the sentinels used for property tree keys.
    json_document_to_contract(FIXTURE, true, platform_version()).expect("fixture validates");
    for (text, bytes) in [("", vec![]), ("\0", vec![]), ("héllo", vec![0, 1])] {
        let document = build(
            &contract,
            "payloadValues",
            vec![
                ("bytes", Value::Bytes(bytes)),
                ("text", Value::Text(text.to_string())),
            ],
            OWNER_1,
            1,
        );
        insert(&drive, &contract, "payloadValues", &document, true).expect("insert payload");
        let query = query(&contract, "payloadValues", vec![], Some(10));
        let (serialized, _, _) = query
            .execute_raw_results_no_proof(&drive, None, None, platform_version())
            .expect("a covering flat scan serializes its payload");
        assert_eq!(serialized.len(), 1);
        let decoded = Document::from_bytes(&serialized[0], query.document_type, platform_version())
            .expect("deserialize response");
        assert_eq!(decoded.properties(), document.properties());
        let (proof, _) = query
            .clone()
            .execute_with_proof(&drive, None, None, platform_version())
            .expect("prove payload");
        let (_, verified) = query
            .verify_proof(&proof, platform_version())
            .expect("verify payload");
        assert_eq!(verified.len(), 1);
        assert_eq!(verified[0].properties(), document.properties());

        let mut wrong = document.clone();
        let mut properties = wrong.properties().clone();
        properties.insert(
            "text".to_string(),
            Value::Text(if text.is_empty() { "\0" } else { "" }.to_string()),
        );
        wrong.set_properties(properties);
        assert!(
            delete(&drive, &contract, "payloadValues", wrong, true).is_err(),
            "empty and NUL strings must have different commitments"
        );
        delete(&drive, &contract, "payloadValues", document, true).expect("delete exact payload");
    }
    assert_grovedb_is_consistent(&drive);
}

#[test]
fn should_refuse_serializing_an_incomplete_flat_scan() {
    let (drive, contract) = setup();
    let document = build(
        &contract,
        "tagged",
        vec![("tag", Value::Text("hello".to_string()))],
        OWNER_1,
        1,
    );
    insert(&drive, &contract, "tagged", &document, true).expect("insert tagged document");
    let query = query(&contract, "tagged", vec![], Some(10));
    let error = query
        .execute_raw_results_no_proof(&drive, None, None, platform_version())
        .expect_err("the flat projection cannot assert that the stored tag is absent");
    assert!(
        error.to_string().contains("does not cover every property"),
        "{error}"
    );

    // A proof still exposes an explicit projection, while querying the
    // covering tag index may return a complete serialized document.
    let (proof, _) = query
        .clone()
        .execute_with_proof(&drive, None, None, platform_version())
        .expect("prove partial flat projection");
    let (_, projected) = query
        .verify_proof(&proof, platform_version())
        .expect("verify projection");
    assert_eq!(projected.len(), 1);
    assert_eq!(projected[0].owner_id(), document.owner_id());
    assert!(!projected[0].properties().contains_key("tag"));

    let covering = self::query(
        &contract,
        "tagged",
        vec![equal("tag", Value::Text("hello".to_string()))],
        Some(10),
    );
    let (serialized, _, _) = covering
        .execute_raw_results_no_proof(&drive, None, None, platform_version())
        .expect("the covering index can serialize the tag");
    let decoded = Document::from_bytes(&serialized[0], covering.document_type, platform_version())
        .expect("deserialize covering response");
    assert_eq!(decoded.properties(), document.properties());
}

/// A `note` document of `body` on the fixture post by `owner`.
fn build_note(contract: &DataContract, body: &[u8], owner: [u8; 32], seed: u64) -> Document {
    build(
        contract,
        "note",
        vec![
            ("postId", Value::Identifier(POST)),
            ("body", Value::Bytes(body.to_vec())),
        ],
        owner,
        seed,
    )
}

/// A variable-width LAST component (a byte array with no `minItems`): an
/// empty value contributes no key bytes, so the entry is keyed by the
/// leading component alone and must still synthesize as an empty byte
/// array rather than the tree-key null sentinel. Ranges on the last
/// component are lowered against the key itself (no padding), in both
/// directions of the bound, with proof parity.
#[test]
fn variable_width_last_component_ranges_and_empty_values() {
    let (drive, contract) = setup();
    let bodies: [&[u8]; 4] = [b"", b"apple", b"banana", b"cherry"];
    for (seed, body) in bodies.iter().enumerate() {
        insert(
            &drive,
            &contract,
            "note",
            &build_note(&contract, body, OWNER_1, seed as u64 + 1),
            true,
        )
        .expect("insert note");
    }
    insert(
        &drive,
        &contract,
        "note",
        &build_note(&contract, b"zebra", OWNER_2, 9),
        true,
    )
    .expect("insert another owner's note");
    assert_commitment_item(
        entry(&drive, &contract, "note", &[("postId", &POST)], &OWNER_1),
        "an empty body keys the entry by the owner alone",
    );

    let run = |clauses: Vec<WhereClause>, what: &str| -> Vec<Vec<u8>> {
        let mut clauses = clauses;
        clauses.insert(0, equal("postId", Value::Identifier(POST)));
        clauses.insert(1, equal("$ownerId", Value::Identifier(OWNER_1)));
        let query = query_ordered(&contract, "note", clauses, vec![("body", true)], Some(10));
        let outcome = drive
            .query_documents(query.clone(), None, false, None, None)
            .unwrap_or_else(|e| panic!("{what}: query executes: {e}"));
        let bodies: Vec<Vec<u8>> = outcome
            .documents()
            .iter()
            .map(|document| payload_bytes(document, "body"))
            .collect();
        let (proof, _) = query
            .clone()
            .execute_with_proof(&drive, None, None, platform_version())
            .unwrap_or_else(|e| panic!("{what}: proof generation: {e}"));
        let (_root, verified) = query
            .verify_proof(proof.as_slice(), platform_version())
            .unwrap_or_else(|e| panic!("{what}: proof verification: {e}"));
        let verified_bodies: Vec<Vec<u8>> = verified
            .iter()
            .map(|document| payload_bytes(document, "body"))
            .collect();
        assert_eq!(
            verified_bodies, bodies,
            "{what}: proved and unproved synthesis agree"
        );
        bodies
    };
    let between = |low: &[u8], high: &[u8], operator: WhereOperator| WhereClause {
        field: "body".to_string(),
        operator,
        value: Value::Array(vec![
            Value::Bytes(low.to_vec()),
            Value::Bytes(high.to_vec()),
        ]),
    };
    let bound = |operator: WhereOperator, value: &[u8]| WhereClause {
        field: "body".to_string(),
        operator,
        value: Value::Bytes(value.to_vec()),
    };

    assert_eq!(
        run(vec![], "every body of the owner"),
        bodies.iter().map(|body| body.to_vec()).collect::<Vec<_>>(),
        "the empty body is the owner's first key and comes back as an empty array"
    );
    assert_eq!(
        run(vec![bound(WhereOperator::LessThan, b"b")], "bodies below b"),
        vec![Vec::new(), b"apple".to_vec()]
    );
    assert_eq!(
        run(
            vec![between(b"b", b"cz", WhereOperator::Between)],
            "bodies between b and cz"
        ),
        vec![b"banana".to_vec(), b"cherry".to_vec()]
    );
    assert_eq!(
        run(
            vec![between(
                b"apple",
                b"cherry",
                WhereOperator::BetweenExcludeBounds
            )],
            "bodies strictly between apple and cherry"
        ),
        vec![b"banana".to_vec()]
    );
    assert_eq!(
        run(
            vec![bound(WhereOperator::GreaterThanOrEquals, b"cherry")],
            "bodies from cherry"
        ),
        vec![b"cherry".to_vec()],
        "another owner's later body sits under a different leading component"
    );

    assert_grovedb_is_consistent(&drive);
}

/// An `in` clause on a composite tail: on the LAST component it addresses
/// each key directly, on a leading component it covers every key under
/// each value; both synthesize the components off the member key with
/// proof parity.
#[test]
fn composite_tail_in_clauses_on_last_and_leading_components() {
    let (drive, contract) = setup();
    for (seed, body) in [b"apple".as_slice(), b"banana", b"cherry"]
        .iter()
        .enumerate()
    {
        insert(
            &drive,
            &contract,
            "note",
            &build_note(&contract, body, OWNER_1, seed as u64 + 1),
            true,
        )
        .expect("insert note");
    }
    for (kind, owner, seed) in [(3u64, OWNER_1, 11u64), (5, OWNER_2, 12), (7, OWNER_3, 13)] {
        insert(
            &drive,
            &contract,
            "reaction",
            &build_reaction(&contract, kind, owner, seed),
            true,
        )
        .expect("insert reaction");
    }

    let notes = query_ordered(
        &contract,
        "note",
        vec![
            equal("postId", Value::Identifier(POST)),
            equal("$ownerId", Value::Identifier(OWNER_1)),
            WhereClause {
                field: "body".to_string(),
                operator: WhereOperator::In,
                value: Value::Array(vec![
                    Value::Bytes(b"cherry".to_vec()),
                    Value::Bytes(b"apple".to_vec()),
                ]),
            },
        ],
        vec![("body", true)],
        Some(10),
    );
    let outcome = drive
        .query_documents(notes.clone(), None, false, None, None)
        .expect("in on the last component executes");
    let bodies: Vec<Vec<u8>> = outcome
        .documents()
        .iter()
        .map(|document| payload_bytes(document, "body"))
        .collect();
    assert_eq!(bodies, vec![b"apple".to_vec(), b"cherry".to_vec()]);
    let (proof, _) = notes
        .clone()
        .execute_with_proof(&drive, None, None, platform_version())
        .expect("proof generation");
    let (_root, verified) = notes
        .verify_proof(proof.as_slice(), platform_version())
        .expect("proof verification");
    let verified_bodies: Vec<Vec<u8>> = verified
        .iter()
        .map(|document| payload_bytes(document, "body"))
        .collect();
    assert_eq!(verified_bodies, bodies);

    let reactions = query_ordered(
        &contract,
        "reaction",
        vec![
            equal("postId", Value::Identifier(POST)),
            WhereClause {
                field: "kind".to_string(),
                operator: WhereOperator::In,
                value: Value::Array(vec![Value::U64(7), Value::U64(3)]),
            },
        ],
        vec![("kind", true)],
        Some(10),
    );
    let outcome = drive
        .query_documents(reactions.clone(), None, false, None, None)
        .expect("in on a leading component executes");
    let kinds_and_owners: Vec<(u64, [u8; 32])> = outcome
        .documents()
        .iter()
        .map(|document| {
            (
                document
                    .properties()
                    .get("kind")
                    .and_then(|value| value.to_integer::<u64>().ok())
                    .expect("kind decoded"),
                document.owner_id().to_buffer(),
            )
        })
        .collect();
    assert_eq!(kinds_and_owners, vec![(3, OWNER_1), (7, OWNER_3)]);
    let (proof, _) = reactions
        .clone()
        .execute_with_proof(&drive, None, None, platform_version())
        .expect("proof generation");
    let (_root, verified) = reactions
        .verify_proof(proof.as_slice(), platform_version())
        .expect("proof verification");
    assert_eq!(verified.len(), 2);

    assert_grovedb_is_consistent(&drive);
}

/// Exercise the shipped system schema, not just the generic feature fixture.
#[test]
fn should_query_and_relogin_with_the_app_connect_system_contract() {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = platform_version();
    let contract = load_system_data_contract(SystemDataContract::AppConnect, pv)
        .expect("app-connect system contract");
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("apply system contract");

    let original = build_login_response(
        &contract,
        REQUEST_HASH,
        WALLET_KEY_1,
        cipher(0xC1, 60),
        OWNER_1,
        1,
    );
    let other_owner = build_login_response(
        &contract,
        REQUEST_HASH,
        WALLET_KEY_2,
        cipher(0xC2, 572),
        OWNER_2,
        2,
    );
    for document in [&other_owner, &original] {
        insert(&drive, &contract, "loginKeyResponse", document, true)
            .expect("an earlier response by another identity cannot squat the request");
    }
    let duplicate = build_login_response(
        &contract,
        REQUEST_HASH,
        WALLET_KEY_3,
        cipher(0xC3, 92),
        OWNER_1,
        3,
    );
    assert!(
        insert(&drive, &contract, "loginKeyResponse", &duplicate, true).is_err(),
        "each owner may answer a request only once"
    );

    let by_request = query(
        &contract,
        "loginKeyResponse",
        vec![equal(
            "appEphemeralPubKeyHash",
            Value::Bytes(REQUEST_HASH.to_vec()),
        )],
        Some(10),
    );
    let (proof, _) = by_request
        .clone()
        .execute_with_proof(&drive, None, None, pv)
        .expect("request proof");
    let (_, documents) = by_request
        .clone()
        .verify_proof(&proof, pv)
        .expect("verify request proof");
    assert_eq!(documents.len(), 2);
    for document in documents {
        let expected = if document.owner_id().to_buffer() == OWNER_1 {
            &original
        } else {
            &other_owner
        };
        assert_eq!(document.properties(), expected.properties());
    }

    assert!(
        delete(&drive, &contract, "loginKeyResponse", duplicate, true).is_err(),
        "deletion must supply the original ciphertext and wallet key"
    );
    delete(&drive, &contract, "loginKeyResponse", original, true).expect("delete old response");
    let renewed = build_login_response(
        &contract,
        OTHER_REQUEST_HASH,
        WALLET_KEY_3,
        cipher(0xC3, 572),
        OWNER_1,
        4,
    );
    insert(&drive, &contract, "loginKeyResponse", &renewed, true).expect("publish re-login");

    assert!(login_entry(&drive, &contract, REQUEST_HASH, OWNER_1).is_none());
    assert!(login_entry(&drive, &contract, REQUEST_HASH, OWNER_2).is_some());
    assert!(login_entry(&drive, &contract, OTHER_REQUEST_HASH, OWNER_1).is_some());
    let (proof, _) = by_request
        .clone()
        .execute_with_proof(&drive, None, None, pv)
        .expect("old request proof");
    let (_, documents) = by_request
        .verify_proof(&proof, pv)
        .expect("verify old request");
    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].owner_id().to_buffer(), OWNER_2);

    let new_request = query(
        &contract,
        "loginKeyResponse",
        vec![
            equal(
                "appEphemeralPubKeyHash",
                Value::Bytes(OTHER_REQUEST_HASH.to_vec()),
            ),
            equal("$ownerId", Value::Identifier(OWNER_1)),
        ],
        Some(1),
    );
    let (proof, _) = new_request
        .clone()
        .execute_with_proof(&drive, None, None, pv)
        .expect("new request proof");
    let (_, documents) = new_request
        .verify_proof(&proof, pv)
        .expect("verify new request");
    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].properties(), renewed.properties());
    assert_grovedb_is_consistent(&drive);
}
