//! Regression coverage for cursors on an IN level followed by a range level.

#![cfg(all(feature = "server", feature = "verify"))]

use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::platform_value::{platform_value, Identifier};
use dpp::prelude::DataContract;
use dpp::util::cbor_serializer;
use dpp::version::PlatformVersion;
use drive::query::DriveDocumentQuery;
use drive::util::object_size_info::{DocumentAndContractInfo, DocumentInfo, OwnedDocumentInfo};
use drive::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use serde_json::json;

#[test]
fn should_activate_compound_cursor_lowering_only_at_protocol_v14() {
    for protocol_version in 1..14 {
        let version = PlatformVersion::get(protocol_version).expect("released version");
        assert_eq!(
            version
                .drive
                .methods
                .document
                .query
                .non_primary_key_path_query,
            0
        );
        assert_eq!(
            version
                .drive
                .methods
                .document
                .query
                .non_primary_key_single_in_path_query,
            0
        );
    }
    let query_versions = &PlatformVersion::latest().drive.methods.document.query;
    assert_eq!(query_versions.non_primary_key_path_query, 1);
    assert_eq!(query_versions.non_primary_key_single_in_path_query, 1);
}

fn assert_compound_cursor_pages(ascending: bool, included: bool, prove: bool) {
    for unique in [false, true] {
        assert_compound_cursor_pages_for_index(ascending, included, prove, unique);
    }
}

fn assert_compound_cursor_pages_for_index(
    ascending: bool,
    included: bool,
    prove: bool,
    unique: bool,
) {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(None);
    let contract = DataContract::from_value(
        platform_value!({
            "$formatVersion": "0",
            "id": "BZUodcFoFL6KvnonehrnMVggTvCe8W5MiRnZuqLb6M54",
            "version": 1,
            "ownerId": "GZVdTnLFAN2yE9rLeCHBDBCr7YQgmXJuoExkY347j7Z5",
            "documentSchemas": {
                "row": {
                    "type": "object",
                    "indices": [{"name": "ab", "unique": unique, "properties": [{"a": "asc"}, {"b": "asc"}]}],
                    "properties": {
                        "a": {"type": "integer", "position": 0},
                        "b": {"type": "integer", "position": 1}
                    },
                    "required": ["a", "b"],
                    "additionalProperties": false
                }
            }
        }),
        false,
        platform_version,
    )
    .expect("should create compound-index contract");
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            None,
            None,
            platform_version,
        )
        .expect("should apply contract");
    let document_type = contract.document_type_for_name("row").expect("row type");

    // Every outer branch has values below, at and above the inner cursor.
    // Duplicate b=5 values exercise the document-id tie breaker as well.
    let mut rows: Vec<(u8, u8, Document)> = Vec::new();
    for a in 0u8..=4 {
        for b in [0u8, 1, 3, 5, 5, 7, 9] {
            if unique && rows.iter().any(|row| row.0 == a && row.1 == b) {
                continue;
            }
            let seed = rows.len() as u8 + 1;
            let mut document = document_type
                .create_document_from_data(
                    platform_value!({"a": a, "b": b}),
                    Identifier::from([100; 32]),
                    1,
                    1,
                    [seed; 32],
                    platform_version,
                )
                .expect("should create row");
            document.set_id(Identifier::from([seed; 32]));
            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentInfo::DocumentRefInfo((&document, None)),
                            owner_id: None,
                        },
                        contract: &contract,
                        document_type,
                    },
                    true,
                    BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                    None,
                )
                .expect("should insert row");
            rows.push((a, b, document));
        }
    }
    let root_hash = drive
        .grove
        .root_hash(None, &platform_version.drive.grove_version)
        .unwrap()
        .expect("root hash");

    for inner_ascending in [ascending, !ascending] {
        let order = if ascending { "asc" } else { "desc" };
        let inner_order = if inner_ascending { "asc" } else { "desc" };
        let compare = |left: &(u8, u8, Document), right: &(u8, u8, Document)| {
            let outer = left.0.cmp(&right.0);
            let inner = (left.1, left.2.id()).cmp(&(right.1, right.2.id()));
            let outer = if ascending { outer } else { outer.reverse() };
            let inner = if inner_ascending {
                inner
            } else {
                inner.reverse()
            };
            outer.then(inner)
        };

        // Unsorted IN values check traversal order. Removing 2 checks a
        // cursor whose outer value is absent from the selected branches.
        for in_values in [vec![3u8, 1, 4, 2], vec![4, 1, 3]] {
            for cursor in rows.iter().filter(|(a, b, _)| {
                // First/middle/last outer branches, both duplicate cursor
                // values, and a cursor excluded by the b > 0 predicate.
                (1..=4).contains(a) && (*b == 5 || *b == 0)
            }) {
                for limit in [4usize, 100] {
                    // GroveDB deliberately charges empty subqueries against
                    // the page's traversal budget. Use the full-page assertion
                    // for those cases; assert a filled short page when the
                    // cursor's id subtree still has a matching document (or
                    // the outer cursor branch is not selected at all).
                    let cursor_subtree_has_results = rows.iter().any(|row| {
                        row.0 == cursor.0
                            && row.1 == cursor.1
                            && row.1 > 0
                            && (compare(row, cursor).is_gt()
                                || (included && compare(row, cursor).is_eq()))
                    });
                    if limit == 4 && in_values.contains(&cursor.0) && !cursor_subtree_has_results {
                        continue;
                    }
                    let mut expected_rows: Vec<_> = rows
                        .iter()
                        .filter(|row| in_values.contains(&row.0) && row.1 > 0)
                        .filter(|row| {
                            let ordering = compare(row, cursor);
                            ordering.is_gt() || (included && ordering.is_eq())
                        })
                        .collect();
                    expected_rows.sort_by(|left, right| compare(left, right));
                    let expected: Vec<_> = expected_rows
                        .into_iter()
                        .take(limit)
                        .map(|row| row.2.id())
                        .collect();

                    let mut query_value = json!({
                        "where": [["a", "in", in_values], ["b", ">", 0]],
                        "orderBy": [["a", order], ["b", inner_order]],
                        "limit": limit
                    });
                    query_value[if included { "startAt" } else { "startAfter" }] =
                        json!(bs58::encode(cursor.2.id().as_slice()).into_string());
                    let context = format!(
                        "unique={unique}, a={order}, b={inner_order}, included={included}, prove={prove}, \
                         IN={in_values:?}, cursor=({}, {}, {}), limit={limit}",
                        cursor.0,
                        cursor.1,
                        cursor.2.id()
                    );
                    let query_bytes =
                        cbor_serializer::serializable_value_to_cbor(&query_value, None)
                            .expect("should serialize query");
                    let query = DriveDocumentQuery::from_cbor(
                        &query_bytes,
                        &contract,
                        document_type,
                        &drive.config,
                        platform_version,
                    )
                    .unwrap_or_else(|error| panic!("{context}: {error}"));
                    let results = if prove {
                        // This generates the proof and runs the client-side
                        // verifier. Compare its complete page to the oracle,
                        // rather than just comparing the two execution paths.
                        let (proof_root_hash, results, _) = query
                            .execute_with_proof_only_get_elements(
                                &drive,
                                None,
                                None,
                                platform_version,
                            )
                            .unwrap_or_else(|error| panic!("{context}: {error}"));
                        assert_eq!(proof_root_hash, root_hash, "{context}");
                        results
                    } else {
                        query
                            .execute_raw_results_no_proof(&drive, None, None, platform_version)
                            .unwrap_or_else(|error| panic!("{context}: {error}"))
                            .0
                    };
                    let actual: Vec<_> = results
                        .iter()
                        .map(|bytes| {
                            Document::from_bytes(bytes, document_type, platform_version)
                                .expect("should deserialize result")
                                .id()
                        })
                        .collect();
                    assert_eq!(actual, expected, "{context}");
                }
            }
        }
    }
}

#[test]
fn should_preserve_compound_cursor_branches_ascending_start_at_without_proof() {
    assert_compound_cursor_pages(true, true, false);
}

#[test]
fn should_preserve_compound_cursor_branches_ascending_start_after_without_proof() {
    assert_compound_cursor_pages(true, false, false);
}

#[test]
fn should_preserve_compound_cursor_branches_descending_start_at_without_proof() {
    assert_compound_cursor_pages(false, true, false);
}

#[test]
fn should_preserve_compound_cursor_branches_descending_start_after_without_proof() {
    assert_compound_cursor_pages(false, false, false);
}

#[test]
fn should_verify_complete_compound_cursor_page_ascending_start_at() {
    assert_compound_cursor_pages(true, true, true);
}

#[test]
fn should_verify_complete_compound_cursor_page_ascending_start_after() {
    assert_compound_cursor_pages(true, false, true);
}

#[test]
fn should_verify_complete_compound_cursor_page_descending_start_at() {
    assert_compound_cursor_pages(false, true, true);
}

#[test]
fn should_verify_complete_compound_cursor_page_descending_start_after() {
    assert_compound_cursor_pages(false, false, true);
}
