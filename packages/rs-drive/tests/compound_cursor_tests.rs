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
    // v14 activates the new lowering through the parent dispatcher; its first
    // implementation remains v0 because it is unreachable in older protocols.
    assert_eq!(query_versions.non_primary_key_single_in_path_query, 0);
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
    // Duplicate b=5 values exercise the document-id tie breaker within the
    // cursor's index key.
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
                    // A row is on the page when it sorts after the cursor (or
                    // at it for startAt); every page is full while rows
                    // remain, whatever the cursor's branch holds after it.
                    let after_cursor = |row: &(u8, u8, Document)| {
                        compare(row, cursor).is_gt() || (included && compare(row, cursor).is_eq())
                    };
                    let mut expected_rows: Vec<_> = rows
                        .iter()
                        .filter(|row| in_values.contains(&row.0) && row.1 > 0)
                        .filter(|row| after_cursor(row))
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

// ---------------------------------------------------------------------------
// Left-over-level coverage: cursors whose refinement continues below the
// clause levels (a third index property, or a non-unique id level under an
// `In` or equality level). Every case compares the executed page — raw and
// proof-verified — against an independently ordered oracle, and paginates
// with `startAfter` until exhaustion to catch both omissions and repeats.
// ---------------------------------------------------------------------------

struct LeftOverFixture {
    drive: drive::drive::Drive,
    contract: DataContract,
    /// `(a, b, c)` values with the inserted document.
    rows: Vec<([u8; 3], Document)>,
}

fn setup_left_over_fixture(
    properties: &[(&str, &str)],
    unique: bool,
    rows: &[[u8; 3]],
) -> LeftOverFixture {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(None);
    let index_properties: Vec<dpp::platform_value::Value> = properties
        .iter()
        .map(|(name, direction)| platform_value!({ *name: *direction }))
        .collect();
    let contract = DataContract::from_value(
        platform_value!({
            "$formatVersion": "0",
            "id": "BZUodcFoFL6KvnonehrnMVggTvCe8W5MiRnZuqLb6M54",
            "version": 1,
            "ownerId": "GZVdTnLFAN2yE9rLeCHBDBCr7YQgmXJuoExkY347j7Z5",
            "documentSchemas": {
                "row": {
                    "type": "object",
                    "indices": [{"name": "abc", "unique": unique, "properties": index_properties}],
                    "properties": {
                        "a": {"type": "integer", "position": 0},
                        "b": {"type": "integer", "position": 1},
                        "c": {"type": "integer", "position": 2}
                    },
                    "required": ["a", "b", "c"],
                    "additionalProperties": false
                }
            }
        }),
        false,
        platform_version,
    )
    .expect("should create left-over contract");
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
    let mut inserted = Vec::new();
    for (index, values) in rows.iter().enumerate() {
        let seed = index as u8 + 1;
        let mut document = document_type
            .create_document_from_data(
                platform_value!({"a": values[0], "b": values[1], "c": values[2]}),
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
        inserted.push((*values, document));
    }
    LeftOverFixture {
        drive,
        contract,
        rows: inserted,
    }
}

/// Oracle ordering: `order` lists `(property index, ascending)` for every
/// index level in index order; ids tie-break in the last level's direction.
fn left_over_oracle(
    fixture: &LeftOverFixture,
    filter: impl Fn(&[u8; 3]) -> bool,
    order: &[(usize, bool)],
) -> Vec<Identifier> {
    left_over_oracle_rows(fixture, filter, order)
        .into_iter()
        .map(|(_, id)| id)
        .collect()
}

fn left_over_oracle_rows(
    fixture: &LeftOverFixture,
    filter: impl Fn(&[u8; 3]) -> bool,
    order: &[(usize, bool)],
) -> Vec<([u8; 3], Identifier)> {
    let mut rows: Vec<&([u8; 3], Document)> = fixture
        .rows
        .iter()
        .filter(|(values, _)| filter(values))
        .collect();
    rows.sort_by(|left, right| {
        for (property, ascending) in order {
            let ordering = left.0[*property].cmp(&right.0[*property]);
            let ordering = if *ascending {
                ordering
            } else {
                ordering.reverse()
            };
            if ordering.is_ne() {
                return ordering;
            }
        }
        let ordering = left.1.id().cmp(&right.1.id());
        if order.last().is_none_or(|(_, ascending)| *ascending) {
            ordering
        } else {
            ordering.reverse()
        }
    });
    rows.into_iter()
        .map(|(values, document)| (*values, document.id()))
        .collect()
}

/// The oracle page for a cursor at `position` of `full`: the matching rows
/// after it (or at it for startAt).
fn left_over_expected_page(
    full: &[([u8; 3], Identifier)],
    matching: &[Identifier],
    position: usize,
    included: bool,
) -> Vec<Identifier> {
    let from = if included { position } else { position + 1 };
    full[from..]
        .iter()
        .filter(|(_, id)| matching.contains(id))
        .map(|(_, id)| *id)
        .collect()
}

fn left_over_page(
    fixture: &LeftOverFixture,
    where_clauses: serde_json::Value,
    order_by: serde_json::Value,
    limit: usize,
    cursor: Option<(Identifier, bool)>,
    prove: bool,
) -> Vec<Identifier> {
    left_over_page_with_offset(fixture, where_clauses, order_by, limit, None, cursor, prove)
}

fn left_over_page_with_offset(
    fixture: &LeftOverFixture,
    where_clauses: serde_json::Value,
    order_by: serde_json::Value,
    limit: usize,
    offset: Option<usize>,
    cursor: Option<(Identifier, bool)>,
    prove: bool,
) -> Vec<Identifier> {
    let platform_version = PlatformVersion::latest();
    let document_type = fixture
        .contract
        .document_type_for_name("row")
        .expect("row type");
    let mut query_value = json!({"where": where_clauses, "orderBy": order_by, "limit": limit});
    if let Some(offset) = offset {
        query_value["offset"] = json!(offset);
    }
    if let Some((id, included)) = cursor {
        query_value[if included { "startAt" } else { "startAfter" }] =
            json!(bs58::encode(id.as_slice()).into_string());
    }
    let query_bytes = cbor_serializer::serializable_value_to_cbor(&query_value, None)
        .expect("should serialize query");
    let query = DriveDocumentQuery::from_cbor(
        &query_bytes,
        &fixture.contract,
        document_type,
        &fixture.drive.config,
        platform_version,
    )
    .unwrap_or_else(|error| panic!("{query_value}: {error}"));
    let results = if prove {
        query
            .execute_with_proof_only_get_elements(&fixture.drive, None, None, platform_version)
            .unwrap_or_else(|error| panic!("{query_value}: {error}"))
            .1
    } else {
        query
            .execute_raw_results_no_proof(&fixture.drive, None, None, platform_version)
            .unwrap_or_else(|error| panic!("{query_value}: {error}"))
            .0
    };
    results
        .iter()
        .map(|bytes| {
            Document::from_bytes(bytes, document_type, platform_version)
                .expect("should deserialize result")
                .id()
        })
        .collect()
}

/// Walks every page with `startAfter` on the previous page's last row.
fn left_over_walk(
    fixture: &LeftOverFixture,
    where_clauses: serde_json::Value,
    order_by: serde_json::Value,
    limit: usize,
    prove: bool,
) -> Vec<Identifier> {
    let mut all = Vec::new();
    let mut cursor = None;
    for _ in 0..100 {
        let page = left_over_page(
            fixture,
            where_clauses.clone(),
            order_by.clone(),
            limit,
            cursor,
            prove,
        );
        let Some(last) = page.last() else {
            break;
        };
        cursor = Some((*last, false));
        all.extend(page);
    }
    all
}

/// Every row (inside or outside the predicate) as a startAt/startAfter
/// cursor, raw and proven, must yield exactly the oracle page.
fn assert_cursors_over_all_rows(
    fixture: &LeftOverFixture,
    where_clauses: serde_json::Value,
    order_by: serde_json::Value,
    filter: impl Fn(&[u8; 3]) -> bool,
    order: &[(usize, bool)],
) {
    let full = left_over_oracle_rows(fixture, |_| true, order);
    let matching = left_over_oracle(fixture, filter, order);
    for prove in [false, true] {
        for (position, (_, id)) in full.iter().enumerate() {
            for included in [true, false] {
                let expected = left_over_expected_page(&full, &matching, position, included);
                let got = left_over_page(
                    fixture,
                    where_clauses.clone(),
                    order_by.clone(),
                    100,
                    Some((*id, included)),
                    prove,
                );
                assert_eq!(
                    got, expected,
                    "{where_clauses} {order_by}: prove={prove}, included={included}, cursor position {position}"
                );
            }
        }
    }
}

fn three_level_rows() -> Vec<[u8; 3]> {
    let mut rows = Vec::new();
    for a in [1u8, 2] {
        for b in [3u8, 5] {
            // Duplicate (b, c) values exercise the id level under c.
            for c in [1u8, 7, 7, 7] {
                rows.push([a, b, c]);
            }
        }
    }
    rows
}

#[test]
fn should_include_start_at_cursor_on_unique_index_with_left_over_level() {
    // Range outside, In inside, and c left over on a unique index: the
    // cursor's c key holds the cursor document, so startAt must keep it.
    let mut rows = Vec::new();
    for a in [1u8, 2] {
        for b in [3u8, 5] {
            for c in [1u8, 7, 9] {
                rows.push([a, b, c]);
            }
        }
    }
    let fixture = setup_left_over_fixture(&[("a", "asc"), ("b", "asc"), ("c", "asc")], true, &rows);
    assert_cursors_over_all_rows(
        &fixture,
        json!([["a", ">", 0], ["b", "in", [3, 5]]]),
        json!([["a", "asc"], ["b", "asc"]]),
        |values| values[0] > 0 && (values[1] == 3 || values[1] == 5),
        &[(0, true), (1, true), (2, true)],
    );
}

#[test]
fn should_include_start_at_cursor_on_unique_in_level_with_left_over_property() {
    // `In` last clause with b left over on a unique index (no inner clause).
    let rows: Vec<[u8; 3]> = [1u8, 2]
        .iter()
        .flat_map(|a| [3u8, 5, 7].iter().map(move |b| [*a, *b, 0]))
        .collect();
    let fixture = setup_left_over_fixture(&[("a", "asc"), ("b", "asc")], true, &rows);
    assert_cursors_over_all_rows(
        &fixture,
        json!([["a", "in", [1, 2]]]),
        json!([["a", "asc"], ["b", "asc"]]),
        |values| values[0] == 1 || values[0] == 2,
        &[(0, true), (1, true)],
    );
}

#[test]
fn should_continue_within_duplicates_on_descending_left_over_level() {
    // c is a descending index property left over below the In level, so its
    // id level walks right to left: a cursor inside a run of equal (b, c)
    // values must continue below the cursor's id, not above it.
    let fixture = setup_left_over_fixture(
        &[("a", "asc"), ("b", "asc"), ("c", "desc")],
        false,
        &three_level_rows(),
    );
    assert_cursors_over_all_rows(
        &fixture,
        json!([["a", ">", 0], ["b", "in", [3, 5]]]),
        json!([["a", "asc"], ["b", "desc"]]),
        |values| values[0] > 0 && (values[1] == 3 || values[1] == 5),
        &[(0, true), (1, false), (2, false)],
    );
}

#[test]
fn should_continue_within_duplicates_on_ascending_left_over_level() {
    let fixture = setup_left_over_fixture(
        &[("a", "asc"), ("b", "asc"), ("c", "asc")],
        false,
        &three_level_rows(),
    );
    assert_cursors_over_all_rows(
        &fixture,
        json!([["a", ">", 0], ["b", "in", [3, 5]]]),
        json!([["a", "asc"], ["b", "asc"]]),
        |values| values[0] > 0 && (values[1] == 3 || values[1] == 5),
        &[(0, true), (1, true), (2, true)],
    );
}

#[test]
fn should_continue_within_duplicates_on_descending_in_level_with_left_over_property() {
    // `In` last clause, b left over and ordered descending, duplicate b values.
    let rows: Vec<[u8; 3]> = [1u8, 2]
        .iter()
        .flat_map(|a| [3u8, 5, 5, 5].iter().map(move |b| [*a, *b, 0]))
        .collect();
    let fixture = setup_left_over_fixture(&[("a", "asc"), ("b", "asc")], false, &rows);
    assert_cursors_over_all_rows(
        &fixture,
        json!([["a", "in", [1, 2]]]),
        json!([["a", "asc"], ["b", "desc"]]),
        |values| values[0] == 1 || values[0] == 2,
        &[(0, true), (1, false)],
    );
}

#[test]
fn should_not_widen_outer_bound_for_cursor_on_the_bound() {
    // Range outside, In inside: a cursor sitting on the strict outer bound
    // must not pull its (excluded) branch into the page.
    let rows: Vec<[u8; 3]> = [3u8, 5, 7]
        .iter()
        .flat_map(|a| [1u8, 2].iter().map(move |b| [*a, *b, 0]))
        .collect();
    let fixture = setup_left_over_fixture(&[("a", "asc"), ("b", "asc")], false, &rows);
    for (operator, filter) in [
        (
            "<",
            (|values: &[u8; 3]| values[0] < 5) as fn(&[u8; 3]) -> bool,
        ),
        ("<=", |values| values[0] <= 5),
        (">", |values| values[0] > 5),
        (">=", |values| values[0] >= 5),
    ] {
        for outer_ascending in [true, false] {
            let outer_order = if outer_ascending { "asc" } else { "desc" };
            assert_cursors_over_all_rows(
                &fixture,
                json!([["a", operator, 5], ["b", "in", [1, 2]]]),
                json!([["a", outer_order], ["b", "asc"]]),
                |values| filter(values) && (values[1] == 1 || values[1] == 2),
                &[(0, outer_ascending), (1, true)],
            );
        }
    }
}

#[test]
fn should_fill_a_limit_one_page_after_any_cursor() {
    // A page of one after every cursor, in both directions: neither the
    // cursor's exhausted id subtree nor its exhausted outer branch may
    // consume the page's only slot (GroveDB charges an empty subquery
    // against the limit, so the lowering must never visit one for the
    // cursor's sake).
    let rows: Vec<[u8; 3]> = vec![[1, 5, 0], [1, 7, 0], [2, 7, 0], [3, 1, 0]];
    let fixture = setup_left_over_fixture(&[("a", "asc"), ("b", "asc")], false, &rows);
    for inner_ascending in [true, false] {
        let inner_order = if inner_ascending { "asc" } else { "desc" };
        let full = left_over_oracle_rows(&fixture, |_| true, &[(0, true), (1, inner_ascending)]);
        let matching = left_over_oracle(
            &fixture,
            |values| (values[0] == 1 || values[0] == 2) && values[1] > 0,
            &[(0, true), (1, inner_ascending)],
        );
        for prove in [false, true] {
            for (position, (_, id)) in full.iter().enumerate() {
                let expected: Vec<_> = left_over_expected_page(&full, &matching, position, false)
                    .into_iter()
                    .take(1)
                    .collect();
                let got = left_over_page(
                    &fixture,
                    json!([["a", "in", [1, 2]], ["b", ">", 0]]),
                    json!([["a", "asc"], ["b", inner_order]]),
                    1,
                    Some((*id, false)),
                    prove,
                );
                assert_eq!(
                    got, expected,
                    "b={inner_order}, prove={prove}, position {position}"
                );
            }
        }
    }
}

#[test]
fn should_apply_an_offset_after_the_start_after_cursor() {
    // A page offset skips rows after the cursor, never the cursor's own
    // reserved slot: the page must start exactly `offset` rows past the
    // cursor in both directions, raw and proven.
    let rows: Vec<[u8; 3]> = vec![[1, 5, 0], [1, 7, 0], [2, 7, 0], [2, 9, 0], [3, 1, 0]];
    let fixture = setup_left_over_fixture(&[("a", "asc"), ("b", "asc")], false, &rows);
    for inner_ascending in [true, false] {
        let inner_order = if inner_ascending { "asc" } else { "desc" };
        let full = left_over_oracle_rows(&fixture, |_| true, &[(0, true), (1, inner_ascending)]);
        let matching = left_over_oracle(
            &fixture,
            |values| (values[0] == 1 || values[0] == 2) && values[1] > 0,
            &[(0, true), (1, inner_ascending)],
        );
        for prove in [false, true] {
            for (position, (_, id)) in full.iter().enumerate() {
                for (offset, limit) in [(1usize, 1usize), (1, 2), (2, 1), (2, 100)] {
                    let expected: Vec<_> =
                        left_over_expected_page(&full, &matching, position, false)
                            .into_iter()
                            .skip(offset)
                            .take(limit)
                            .collect();
                    let got = left_over_page_with_offset(
                        &fixture,
                        json!([["a", "in", [1, 2]], ["b", ">", 0]]),
                        json!([["a", "asc"], ["b", inner_order]]),
                        limit,
                        Some(offset),
                        Some((*id, false)),
                        prove,
                    );
                    assert_eq!(
                        got, expected,
                        "b={inner_order}, prove={prove}, position {position}, offset {offset}, limit {limit}"
                    );
                }
            }
        }
    }
}

#[test]
fn should_continue_within_duplicates_after_an_equality_clause() {
    // Equality last clause with two left-over levels on a non-unique index:
    // a startAfter cursor on the first of two documents sharing (a, b, c)
    // must be followed by the second.
    let fixture = setup_left_over_fixture(
        &[("a", "asc"), ("b", "asc"), ("c", "asc")],
        false,
        &[[1, 3, 5], [1, 3, 5], [1, 4, 1]],
    );
    assert_cursors_over_all_rows(
        &fixture,
        json!([["a", "==", 1]]),
        json!([["b", "asc"]]),
        |values| values[0] == 1,
        &[(0, true), (1, true), (2, true)],
    );
}

#[test]
fn should_not_widen_inner_bound_for_cursor_on_or_past_the_bound() {
    // Duplicate b=5 values sit exactly on the inner clause bounds; a cursor
    // at b=5 must not pull the other b=5 documents into a strict range, and
    // must still paginate within them for an inclusive one.
    let rows: Vec<[u8; 3]> = [1u8, 2]
        .iter()
        .flat_map(|a| [1u8, 3, 5, 5, 5, 7].iter().map(move |b| [*a, *b, 0]))
        .collect();
    let fixture = setup_left_over_fixture(&[("a", "asc"), ("b", "asc")], false, &rows);
    type RowFilter = Box<dyn Fn(&[u8; 3]) -> bool>;
    let predicates: [(&str, RowFilter); 4] = [
        ("<", Box::new(|values| values[1] < 5)),
        ("<=", Box::new(|values| values[1] <= 5)),
        (">", Box::new(|values| values[1] > 5)),
        (">=", Box::new(|values| values[1] >= 5)),
    ];
    for (operator, filter) in predicates {
        for inner_ascending in [true, false] {
            let inner_order = if inner_ascending { "asc" } else { "desc" };
            assert_cursors_over_all_rows(
                &fixture,
                json!([["a", "in", [1, 2]], ["b", operator, 5]]),
                json!([["a", "asc"], ["b", inner_order]]),
                |values| (values[0] == 1 || values[0] == 2) && filter(values),
                &[(0, true), (1, inner_ascending)],
            );
        }
    }
}

#[test]
fn should_not_return_empty_page_while_rows_remain_after_equality_with_limit_one() {
    // An exhausted cursor subtree must not consume the whole page: with
    // limit 1, every startAfter page must still carry the next row.
    let rows: Vec<[u8; 3]> = [1u8, 3, 5, 7].iter().map(|b| [1, *b, 0]).collect();
    let fixture = setup_left_over_fixture(&[("a", "asc"), ("b", "asc")], false, &rows);
    for ascending in [true, false] {
        let expected = left_over_oracle(&fixture, |values| values[0] == 1, &[(1, ascending)]);
        let order = if ascending { "asc" } else { "desc" };
        for prove in [false, true] {
            let got = left_over_walk(
                &fixture,
                json!([["a", "==", 1]]),
                json!([["b", order]]),
                1,
                prove,
            );
            assert_eq!(got, expected, "prove={prove}, order={order}");
        }
    }
}
