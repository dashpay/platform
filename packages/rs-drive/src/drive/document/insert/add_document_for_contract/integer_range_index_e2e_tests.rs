//! End-to-end coverage for integer-range index fan-out: a document is
//! indexed under the start of every window its integer value falls in,
//! those windows are readable through an `IN_INTEGER_RANGE` resolution,
//! updates move the document between windows, and deletion removes every
//! entry. The bottom window of a grid whose starts reach below the
//! property's integer type is clamped to the type's minimum.

use crate::config::DriveConfig;
use crate::drive::document::paths::contract_document_type_path_vec;
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::{
    resolve_integer_range_bucket_clause, DriveDocumentQuery, IntegerRangeGridSpec,
    ResolvedTimeRange, WhereClause, WhereOperator,
};
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContractFactory;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0, DocumentV0Getters, DocumentV0Setters};
use dpp::platform_value::{platform_value, Identifier, Value};
use dpp::prelude::DataContract;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType::QueryKeyElementPairResultType;
use grovedb::{PathQuery, Query, SizedQuery};
use std::collections::BTreeMap;

const OWNER: [u8; 32] = [7; 32];

/// A `listing` type whose `price` (declared by `price_schema`) is bucketed by
/// `integer_range` under a countable `(price, category)` index named
/// `byPriceBand`. `extra_indices` are added as they are.
fn build_listing_contract(
    price_schema: Value,
    integer_range: Value,
    extra_indices: Vec<Value>,
) -> DataContract {
    let mut indices = vec![platform_value!({
        "name": "byPriceBand",
        "properties": [{"price": "asc"}, {"category": "asc"}],
        "integerRange": integer_range,
        "countable": "countable"
    })];
    indices.extend(extra_indices);
    let schemas = platform_value!({
        "listing": {
            "type": "object",
            "properties": {
                "price": price_schema,
                "category": {"type": "string", "maxLength": 63, "position": 1}
            },
            "required": ["price", "category"],
            "indices": Value::Array(indices),
            "additionalProperties": false
        }
    });
    DataContractFactory::new(PlatformVersion::latest().protocol_version)
        .expect("factory")
        .create_with_value_config(Identifier::from([202u8; 32]), 0, schemas, None, None)
        .expect("create contract")
        .data_contract_owned()
}

/// Prices 0..=1_000_000 (a `u32` key), windows of 300 starting every 100.
fn build_overlapping_price_contract() -> DataContract {
    build_listing_contract(
        platform_value!({"type": "integer", "minimum": 0, "maximum": 1_000_000, "position": 0}),
        platform_value!({"on": "price", "range": 300u64, "step": 100u64}),
        vec![],
    )
}

fn setup(contract: &DataContract) -> Drive {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    drive
        .apply_contract(
            contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("apply contract");
    drive
}

fn listing(marker: u8, price: Value, category: &str) -> Document {
    Document::V0(DocumentV0 {
        id: Identifier::from([marker; 32]),
        owner_id: Identifier::from(OWNER),
        properties: BTreeMap::from([
            ("price".to_string(), price),
            ("category".to_string(), Value::Text(category.to_string())),
        ]),
        revision: Some(1),
        ..Default::default()
    })
}

fn insert(drive: &Drive, contract: &DataContract, document: &Document) {
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        document,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(OWNER),
                },
                contract,
                document_type: contract.document_type_for_name("listing").expect("listing"),
            },
            false,
            BlockInfo::default(),
            true,
            None,
            PlatformVersion::latest(),
            None,
        )
        .expect("add document");
}

/// A query for the window of `price` starting at `start`, resolved exactly as
/// the server resolves an `IN_INTEGER_RANGE` clause, with its provenance.
fn window_query<'a>(
    contract: &'a DataContract,
    document_type: DocumentTypeRef<'a>,
    start: Value,
    grid: Option<IntegerRangeGridSpec>,
) -> Result<DriveDocumentQuery<'a>, Error> {
    let (clause, resolved) =
        resolve_integer_range_bucket_clause("price", &start, grid, document_type)?;
    Ok(query_with(contract, document_type, clause, vec![resolved]))
}

fn query_with<'a>(
    contract: &'a DataContract,
    document_type: DocumentTypeRef<'a>,
    clause: WhereClause,
    resolved_time_ranges: Vec<ResolvedTimeRange>,
) -> DriveDocumentQuery<'a> {
    let mut query = DriveDocumentQuery::from_value(
        Value::Map(vec![]),
        contract,
        document_type,
        &DriveConfig::default(),
        PlatformVersion::latest(),
    )
    .expect("build query");
    query
        .internal_clauses
        .equal_clauses
        .insert(clause.field.clone(), clause);
    query.resolved_time_ranges = resolved_time_ranges;
    query
}

fn ids_in_window(drive: &Drive, contract: &DataContract, start: Value) -> Vec<Identifier> {
    let document_type = contract.document_type_for_name("listing").expect("listing");
    window_query(contract, document_type, start, None)
        .expect("the window resolves")
        .execute_raw_results_no_proof(drive, None, None, PlatformVersion::latest())
        .expect("query")
        .0
        .into_iter()
        .map(|serialized| {
            Document::from_bytes(&serialized, document_type, PlatformVersion::latest())
                .expect("document")
                .id()
        })
        .collect()
}

#[test]
fn should_index_a_document_under_every_containing_window_and_delete_them_all() {
    let contract = build_overlapping_price_contract();
    let drive = setup(&contract);
    let document = listing(1, Value::U64(750), "books");
    insert(&drive, &contract, &document);

    for start in [500u64, 600, 700] {
        assert_eq!(
            ids_in_window(&drive, &contract, Value::U64(start)),
            vec![document.id()],
            "the document is in the window starting at {start}"
        );
    }
    assert!(ids_in_window(&drive, &contract, Value::U64(800)).is_empty());
    assert!(ids_in_window(&drive, &contract, Value::U64(400)).is_empty());

    drive
        .delete_document_for_contract(
            document.id(),
            &contract,
            "listing",
            BlockInfo::default(),
            true,
            None,
            PlatformVersion::latest(),
            None,
        )
        .expect("delete document");
    for start in [500u64, 600, 700] {
        assert!(
            ids_in_window(&drive, &contract, Value::U64(start)).is_empty(),
            "window {start} is empty after the delete"
        );
    }
}

#[test]
fn should_move_a_document_between_windows_when_its_value_changes() {
    let contract = build_overlapping_price_contract();
    let drive = setup(&contract);
    let document_type = contract.document_type_for_name("listing").expect("listing");
    let mut document = listing(1, Value::U64(750), "books");
    insert(&drive, &contract, &document);

    let update = |document: &Document| {
        drive
            .update_document_for_contract(
                document,
                &contract,
                document_type,
                Some(OWNER),
                BlockInfo::default(),
                true,
                None,
                None,
                PlatformVersion::latest(),
                None,
            )
            .expect("update document");
    };

    // 750 -> 820 keeps windows 600 and 700, leaves 500 and enters 800.
    document.set("price", Value::U64(820));
    document.set_revision(Some(2));
    update(&document);
    assert!(ids_in_window(&drive, &contract, Value::U64(500)).is_empty());
    for start in [600u64, 700, 800] {
        assert_eq!(
            ids_in_window(&drive, &contract, Value::U64(start)),
            vec![document.id()],
            "window {start} holds the document once"
        );
    }

    // A change below the bucketed level keeps every window and moves the
    // entry to the new category.
    document.set("category", Value::Text("maps".to_string()));
    document.set_revision(Some(3));
    update(&document);
    for start in [600u64, 700, 800] {
        assert_eq!(
            ids_in_window(&drive, &contract, Value::U64(start)),
            vec![document.id()],
            "window {start} still holds the document once"
        );
    }
}

/// A `listing` type whose `category` is optional and whose
/// `(integerRange(price, 300, 100), category)` index skips a listing without
/// one (`skipIfAbsent: true`).
fn build_skip_listing_contract() -> DataContract {
    let schemas = platform_value!({
        "listing": {
            "type": "object",
            "properties": {
                "price": {"type": "integer", "minimum": 0, "maximum": 1_000_000, "position": 0},
                "category": {"type": "string", "minLength": 1, "maxLength": 63, "position": 1}
            },
            "required": ["price"],
            "indices": [{
                "name": "byPriceBand",
                "properties": [{"price": "asc"}, {"category": "asc"}],
                "integerRange": {"on": "price", "range": 300u64, "step": 100u64},
                "skipIfAbsent": true
            }],
            "additionalProperties": false
        }
    });
    DataContractFactory::new(PlatformVersion::latest().protocol_version)
        .expect("factory")
        .create_with_value_config(Identifier::from([205u8; 32]), 0, schemas, None, None)
        .expect("create contract")
        .data_contract_owned()
}

/// The keys directly under `path`.
fn keys_under(drive: &Drive, path: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
    let mut query = Query::new();
    query.insert_all();
    let path_query = PathQuery::new(path, SizedQuery::new(query, None, None));
    let (elements, _) = drive
        .grove_get_raw_path_query(
            &path_query,
            None,
            QueryKeyElementPairResultType,
            &mut vec![],
            &PlatformVersion::latest().drive,
        )
        .expect("read a layer");
    elements
        .to_key_elements()
        .into_iter()
        .map(|(key, _)| key)
        .collect()
}

#[test]
fn should_move_a_listing_into_and_out_of_the_windows_of_a_skip_index() {
    let contract = build_skip_listing_contract();
    let drive = setup(&contract);
    let document_type = contract.document_type_for_name("listing").expect("listing");
    let mut windows_path = contract_document_type_path_vec(contract.id_ref().as_bytes(), "listing");
    windows_path.push(b"price#300#100".to_vec());
    let books_in_window = |start: u64| -> Vec<Identifier> {
        let mut query = window_query(&contract, document_type, Value::U64(start), None)
            .expect("the window resolves");
        query.internal_clauses.equal_clauses.insert(
            "category".to_string(),
            WhereClause {
                field: "category".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("books".to_string()),
            },
        );
        query
            .execute_raw_results_no_proof(&drive, None, None, PlatformVersion::latest())
            .expect("query")
            .0
            .into_iter()
            .map(|serialized| {
                Document::from_bytes(&serialized, document_type, PlatformVersion::latest())
                    .expect("document")
                    .id()
            })
            .collect()
    };
    let update = |document: &Document| {
        drive
            .update_document_for_contract(
                document,
                &contract,
                document_type,
                Some(OWNER),
                BlockInfo::default(),
                true,
                None,
                None,
                PlatformVersion::latest(),
                None,
            )
            .expect("update document");
    };

    // Without a category the listing builds no window at all.
    let mut document = listing(1, Value::U64(750), "books");
    document.remove("category");
    insert(&drive, &contract, &document);
    assert!(keys_under(&drive, windows_path.clone()).is_empty());

    // Adding the category enters the listing into every containing window.
    document.set("category", Value::Text("books".to_string()));
    document.set_revision(Some(2));
    update(&document);
    assert_eq!(keys_under(&drive, windows_path.clone()).len(), 3);
    for start in [500u64, 600, 700] {
        assert_eq!(
            books_in_window(start),
            vec![document.id()],
            "window {start} holds the listing once"
        );
    }

    // Dropping it takes the listing out of every window again.
    document.remove("category");
    document.set_revision(Some(3));
    update(&document);
    for start in [500u64, 600, 700] {
        assert!(
            books_in_window(start).is_empty(),
            "window {start} no longer holds the listing"
        );
    }
    // Nor does it land under a null category: the index skips it.
    for window in keys_under(&drive, windows_path.clone()) {
        let mut category_path = windows_path.clone();
        category_path.extend([window, b"category".to_vec()]);
        assert!(
            !keys_under(&drive, category_path).contains(&Vec::new()),
            "no window keeps the listing under a null category"
        );
    }

    drive
        .delete_document_for_contract(
            document.id(),
            &contract,
            "listing",
            BlockInfo::default(),
            true,
            None,
            PlatformVersion::latest(),
            None,
        )
        .expect("delete a listing the index skips");
}

/// A `listing` type with a UNIQUE `(integerRange(price, 100, 100),
/// category)` index: one listing per category per band of 100.
fn build_unique_listing_contract() -> DataContract {
    let schemas = platform_value!({
        "listing": {
            "type": "object",
            "properties": {
                "price": {"type": "integer", "minimum": 0, "maximum": 1_000_000, "position": 0},
                "category": {"type": "string", "maxLength": 63, "position": 1}
            },
            "required": ["price", "category"],
            "indices": [{
                "name": "byPriceBand",
                "properties": [{"price": "asc"}, {"category": "asc"}],
                "unique": true,
                "integerRange": {"on": "price", "range": 100u64, "step": 100u64}
            }],
            "additionalProperties": false
        }
    });
    DataContractFactory::new(PlatformVersion::latest().protocol_version)
        .expect("factory")
        .create_with_value_config(Identifier::from([204u8; 32]), 0, schemas, None, None)
        .expect("create contract")
        .data_contract_owned()
}

#[test]
fn should_move_a_document_between_windows_of_a_unique_integer_range_index() {
    // A unique index stores the reference AT `…/<window>/<category>/[0]`;
    // a value change that leaves the window inserts under a fresh window and
    // deletes (and prunes) the old one, which a time grid never does.
    let contract = build_unique_listing_contract();
    let drive = setup(&contract);
    let document_type = contract.document_type_for_name("listing").expect("listing");
    let mut document = listing(1, Value::U64(120), "books");
    insert(&drive, &contract, &document);

    let update = |document: &Document| {
        drive
            .update_document_for_contract(
                document,
                &contract,
                document_type,
                Some(OWNER),
                BlockInfo::default(),
                true,
                None,
                None,
                PlatformVersion::latest(),
                None,
            )
            .expect("update document");
    };

    document.set("price", Value::U64(250));
    document.set_revision(Some(2));
    update(&document);
    assert!(ids_in_window(&drive, &contract, Value::U64(100)).is_empty());
    assert_eq!(
        ids_in_window(&drive, &contract, Value::U64(200)),
        vec![document.id()]
    );

    // A change inside the window refreshes the reference in place.
    document.set("price", Value::U64(280));
    document.set_revision(Some(3));
    update(&document);
    assert_eq!(
        ids_in_window(&drive, &contract, Value::U64(200)),
        vec![document.id()]
    );

    // The vacated window takes another listing of the same category.
    let other = listing(2, Value::U64(150), "books");
    insert(&drive, &contract, &other);
    assert_eq!(
        ids_in_window(&drive, &contract, Value::U64(100)),
        vec![other.id()]
    );

    drive
        .delete_document_for_contract(
            document.id(),
            &contract,
            "listing",
            BlockInfo::default(),
            true,
            None,
            PlatformVersion::latest(),
            None,
        )
        .expect("delete document");
    assert!(ids_in_window(&drive, &contract, Value::U64(200)).is_empty());
}

#[test]
fn should_clamp_the_bottom_window_of_a_signed_grid_to_the_type_minimum() {
    // -100..=100 makes an `i8` key: windows starting at -200 and -300 sit
    // below -128 and clamp to it.
    let contract = build_listing_contract(
        platform_value!({"type": "integer", "minimum": -100, "maximum": 100, "position": 0}),
        platform_value!({"on": "price", "range": 300u64, "step": 100u64}),
        vec![],
    );
    let drive = setup(&contract);
    let low = listing(1, Value::I64(-50), "books");
    let high = listing(2, Value::I64(50), "books");
    insert(&drive, &contract, &low);
    insert(&drive, &contract, &high);

    assert_eq!(
        ids_in_window(&drive, &contract, Value::I64(-128)),
        vec![low.id(), high.id()],
        "both values are in the clamped bottom window"
    );
    assert_eq!(
        ids_in_window(&drive, &contract, Value::I64(-100)),
        vec![low.id(), high.id()]
    );
    assert_eq!(
        ids_in_window(&drive, &contract, Value::I64(0)),
        vec![high.id()]
    );

    let document_type = contract.document_type_for_name("listing").expect("listing");
    assert!(
        window_query(&contract, document_type, Value::I64(-200), None).is_err(),
        "a start the type cannot hold names no window"
    );
}

#[test]
fn should_refuse_off_grid_starts_and_ambiguous_grids() {
    let contract = build_listing_contract(
        platform_value!({"type": "integer", "minimum": 0, "maximum": 1_000_000, "position": 0}),
        platform_value!({"on": "price", "range": 100u64, "step": 100u64}),
        vec![platform_value!({
            "name": "byPriceThousands",
            "properties": [{"price": "asc"}, {"category": "asc"}],
            "integerRange": {"on": "price", "range": 1_000u64, "step": 1_000u64}
        })],
    );
    let document_type = contract.document_type_for_name("listing").expect("listing");

    let error = window_query(&contract, document_type, Value::U64(100), None)
        .expect_err("two grids bucket the field");
    assert!(
        matches!(&error, Error::Query(QuerySyntaxError::Unsupported(message)) if message.contains("different integer-range grids")),
        "{error:?}"
    );
    let hundreds = IntegerRangeGridSpec {
        range: 100,
        step: 100,
        phase: 0,
    };
    window_query(&contract, document_type, Value::U64(100), Some(hundreds))
        .expect("naming the grid resolves");
    let error = window_query(&contract, document_type, Value::U64(150), Some(hundreds))
        .expect_err("150 is not a window start");
    assert!(
        matches!(&error, Error::Query(QuerySyntaxError::Unsupported(message)) if message.contains("not a window start")),
        "{error:?}"
    );
    let error = window_query(
        &contract,
        document_type,
        Value::Text("100".to_string()),
        Some(hundreds),
    )
    .expect_err("a start must be an integer");
    assert!(matches!(
        error,
        Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(_))
    ));
}

#[test]
fn should_keep_a_raw_equality_off_the_bucketed_index() {
    let contract = build_overlapping_price_contract();
    let drive = setup(&contract);
    let document_type = contract.document_type_for_name("listing").expect("listing");
    insert(&drive, &contract, &listing(1, Value::U64(700), "books"));

    // `price == 700` without provenance means the raw price, which only a
    // plain index could answer; the bucketed one stores window starts.
    let raw = query_with(
        &contract,
        document_type,
        WhereClause {
            field: "price".to_string(),
            operator: crate::query::WhereOperator::Equal,
            value: Value::U64(700),
        },
        vec![],
    );
    let error = raw
        .execute_raw_results_no_proof(&drive, None, None, PlatformVersion::latest())
        .expect_err("no plain index covers price");
    assert!(
        format!("{error:?}").contains("integerRange"),
        "the refusal names the bucketed indexes: {error:?}"
    );
}

#[test]
fn should_serve_a_proved_window_query() {
    let contract = build_overlapping_price_contract();
    let drive = setup(&contract);
    let document_type = contract.document_type_for_name("listing").expect("listing");
    let document = listing(1, Value::U64(750), "books");
    insert(&drive, &contract, &document);

    let query = window_query(&contract, document_type, Value::U64(600), None).expect("resolves");
    let (proof, _) = query
        .clone()
        .execute_with_proof(&drive, None, None, PlatformVersion::latest())
        .expect("prove");
    let (_, documents) = query
        .verify_proof(&proof, PlatformVersion::latest())
        .expect("verify");
    assert_eq!(
        documents
            .iter()
            .map(|document| document.id())
            .collect::<Vec<_>>(),
        vec![document.id()]
    );
}
