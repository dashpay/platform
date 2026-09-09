//! Worst-case benchmarks for the document-count query paths introduced by
//! `GetDocumentsRequestV1`.
//!
//! The fixture intentionally uses Drive's normal contract application and
//! document insertion path so the resulting GroveDB contains the same primary
//! trees, countable index trees, and range-countable index trees as production.
//!
//! Environment knobs:
//! - `DASH_PLATFORM_COUNT_BENCH_ROWS`: row count to build; defaults to 2,000,000.
//! - `DASH_PLATFORM_COUNT_BENCH_DB`: fixture directory; defaults under `std::env::temp_dir()`.
//! - `DASH_PLATFORM_COUNT_BENCH_REBUILD=1`: remove and rebuild the fixture.
//! - `DASH_PLATFORM_COUNT_BENCH_BATCH_SIZE`: inserts per transaction; defaults to 10,000.

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::{DataContract, DataContractFactory};
use dpp::document::{Document, DocumentV0};
use dpp::identifier::Identifier;
use dpp::platform_value::{platform_value, Value};
use dpp::version::PlatformVersion;
use drive::config::DriveConfig;
use drive::drive::Drive;
use drive::query::{
    CountMode, DocumentCountRequest, DocumentCountResponse, DriveDocumentCountQuery, WhereClause,
    WhereOperator,
};
use drive::util::object_size_info::DocumentInfo::DocumentRefInfo;
use drive::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use drive::util::storage_flags::StorageFlags;
use grovedb::operations::proof::GroveDBProof;
use grovedb::{GroveDb, PathQuery};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

const PROTOCOL_VERSION_V12: u32 = 12;
// Bumped when the on-disk fixture layout changes in a way that
// invalidates a cached `tmp/dash-platform-document-count-bench-v{N}-rows-…`
// directory. v2: countable-terminator value trees are now `CountTree`
// for any countability tier (not just `range_countable`), with
// continuations wrapped `NonCounted`. Old v1 caches were built under
// the previous layout and need to be rebuilt to verify proofs
// against the new code.
const FIXTURE_SCHEMA_VERSION: u32 = 2;
const DEFAULT_ROW_COUNT: u64 = 100_000;
const DEFAULT_BATCH_SIZE: u64 = 10_000;
const BRAND_COUNT: u64 = 100;
const DOCUMENT_TYPE_NAME: &str = "widget";
const READY_MARKER: &str = ".document-count-worst-case-ready";

struct CountBenchFixture {
    drive: Drive,
    data_contract: DataContract,
    drive_config: DriveConfig,
    row_count: u64,
    range_floor: String,
}

impl CountBenchFixture {
    fn load_or_create() -> Self {
        let row_count = row_count();
        let fixture_path = fixture_path(row_count);
        let rebuild = env_flag("DASH_PLATFORM_COUNT_BENCH_REBUILD");
        let ready_marker = fixture_path.join(READY_MARKER);
        let expected_marker = fixture_marker(row_count);

        if rebuild && fixture_path.exists() {
            fs::remove_dir_all(&fixture_path).expect("expected to remove old count bench fixture");
        }

        let data_contract = widget_contract();
        let drive_config = DriveConfig::default();

        if ready_marker.exists()
            && fs::read_to_string(&ready_marker)
                .expect("expected to read count bench fixture marker")
                == expected_marker
        {
            eprintln!(
                "reusing document-count fixture at {} with {} rows",
                fixture_path.display(),
                row_count
            );
            let (drive, _) = Drive::open(&fixture_path, Some(drive_config.clone()))
                .expect("expected to open existing count bench fixture");
            return Self::new(drive, data_contract, drive_config, row_count);
        }

        if fixture_path.exists() {
            fs::remove_dir_all(&fixture_path)
                .expect("expected to remove incomplete count bench fixture");
        }
        fs::create_dir_all(&fixture_path).expect("expected to create count bench fixture dir");

        eprintln!(
            "building document-count fixture at {} with {} rows",
            fixture_path.display(),
            row_count
        );

        let started = Instant::now();
        let platform_version = PlatformVersion::latest();
        let (drive, _) = Drive::open(&fixture_path, Some(drive_config.clone()))
            .expect("expected to open new count bench fixture");

        drive
            .create_initial_state_structure(None, platform_version)
            .expect("expected to create initial state structure");
        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply count bench contract");

        populate_fixture(&drive, &data_contract, row_count, platform_version);
        fs::write(&ready_marker, expected_marker)
            .expect("expected to mark count bench fixture ready");

        eprintln!(
            "built document-count fixture with {} rows in {:.2?}",
            row_count,
            started.elapsed()
        );

        Self::new(drive, data_contract, drive_config, row_count)
    }

    fn new(
        drive: Drive,
        data_contract: DataContract,
        drive_config: DriveConfig,
        row_count: u64,
    ) -> Self {
        let color_count = color_count_for_rows(row_count);
        let range_floor = color_label(color_count / 2);

        Self {
            drive,
            data_contract,
            drive_config,
            row_count,
            range_floor,
        }
    }
}

fn widget_contract() -> DataContract {
    let factory =
        DataContractFactory::new(PROTOCOL_VERSION_V12).expect("expected to create factory");
    let document_schema = platform_value!({
        "type": "object",
        "documentsCountable": true,
        "properties": {
            "brand": {"type": "string", "position": 0, "maxLength": 32},
            "color": {"type": "string", "position": 1, "maxLength": 32},
            "serial": {"type": "integer", "position": 2}
        },
        "required": ["brand", "color", "serial"],
        "indices": [
            {
                "name": "byBrand",
                "properties": [{"brand": "asc"}],
                "countable": "countable"
            },
            {
                "name": "byColor",
                "properties": [{"color": "asc"}],
                "countable": "countable",
                "rangeCountable": true
            },
            {
                "name": "byBrandColor",
                "properties": [{"brand": "asc"}, {"color": "asc"}],
                "countable": "countable",
                "rangeCountable": true
            }
        ],
        "additionalProperties": false
    });
    let schemas = platform_value!({ DOCUMENT_TYPE_NAME: document_schema });

    factory
        .create_with_value_config(Identifier::from([42u8; 32]), 0, schemas, None, None)
        .expect("expected to create count bench data contract")
        .data_contract_owned()
}

fn populate_fixture(
    drive: &Drive,
    data_contract: &DataContract,
    row_count: u64,
    platform_version: &PlatformVersion,
) {
    let document_type = data_contract
        .document_type_for_name(DOCUMENT_TYPE_NAME)
        .expect("expected widget document type");
    let batch_size = batch_size();
    let brands: Vec<String> = (0..BRAND_COUNT).map(brand_label).collect();
    let colors: Vec<String> = (0..color_count_for_rows(row_count))
        .map(color_label)
        .collect();

    let mut next_row = 0;
    while next_row < row_count {
        let end_row = (next_row + batch_size).min(row_count);
        let transaction = drive.grove.start_transaction();

        for row in next_row..end_row {
            let brand = &brands[(row % BRAND_COUNT) as usize];
            let color = &colors[(row / BRAND_COUNT) as usize];
            insert_widget_document(
                drive,
                data_contract,
                document_type,
                row,
                brand,
                color,
                Some(&transaction),
                platform_version,
            );
        }

        drive
            .grove
            .commit_transaction(transaction)
            .value
            .expect("expected count bench insert transaction to commit");

        next_row = end_row;
        if next_row == row_count || next_row % 100_000 == 0 {
            eprintln!("inserted {next_row}/{row_count} count bench rows");
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn insert_widget_document(
    drive: &Drive,
    data_contract: &DataContract,
    document_type: dpp::data_contract::document_type::DocumentTypeRef,
    row: u64,
    brand: &str,
    color: &str,
    transaction: grovedb::TransactionArg,
    platform_version: &PlatformVersion,
) {
    let mut properties = BTreeMap::new();
    properties.insert("brand".to_string(), Value::Text(brand.to_string()));
    properties.insert("color".to_string(), Value::Text(color.to_string()));
    properties.insert("serial".to_string(), Value::U64(row));

    let document: Document = DocumentV0 {
        contract_version: None,
        id: Identifier::from(document_id(row)),
        owner_id: Identifier::from([7u8; 32]),
        properties,
        revision: None,
        created_at: None,
        updated_at: None,
        transferred_at: None,
        created_at_block_height: None,
        updated_at_block_height: None,
        transferred_at_block_height: None,
        created_at_core_block_height: None,
        updated_at_core_block_height: None,
        transferred_at_core_block_height: None,
        creator_id: None,
    }
    .into();

    let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&document, storage_flags)),
                    owner_id: None,
                },
                contract: data_contract,
                document_type,
            },
            false,
            BlockInfo::default(),
            true,
            transaction,
            platform_version,
            None,
        )
        .expect("expected to insert count bench document");
}

fn document_count_worst_case(c: &mut Criterion) {
    let fixture = CountBenchFixture::load_or_create();
    let platform_version = PlatformVersion::latest();
    let brands = all_brand_values();
    let broad_range_floor = Value::Text(fixture.range_floor.clone());

    // One-shot proof-size report. Criterion measures time, but for
    // count-proof work the load-bearing number is bytes-per-proof —
    // an optimization that shaves a merk layer (e.g. the
    // rangeCountable terminator's `[0]` descent) drops proof size
    // linearly with the number of resolved branches while leaving
    // wall-clock per-proof time roughly unchanged on warm caches.
    // Print sizes once at bench setup so reviewers can compare
    // before/after numbers from the same fixture without parsing
    // criterion's HTML output.
    report_proof_sizes(&fixture, &brands, &broad_range_floor, platform_version);

    // Full `(group_by × where_shape)` outcome matrix at the drive
    // layer. Surfaces which combinations:
    // - the drive dispatcher accepts (vs rejects with a typed error)
    // - succeed on the no-proof path
    // - succeed on the prove path
    // - what proof bytes the prove path emits
    //
    // Run once at bench setup so the matrix reflects the current
    // optimization + dispatcher state without needing a separate
    // integration test.
    report_group_by_matrix(&fixture, platform_version);

    // Decoded display of every `group_by = []` proof: the path
    // query that produced it (path, items, subquery) and the
    // verified payload (root hash + count/elements). The path
    // query is the prover-side spec and the verified payload is
    // what `GroveDb::verify_query` / `verify_aggregate_count_query`
    // reconstructs after walking the proof — together they make
    // the proof's *meaning* legible without staring at hex.
    display_proofs(&fixture, platform_version);

    // Decoded display of every `group_by` proof shape in the Count
    // Index Group By Examples chapter (G1a, G1b, G3..G5, G7, G8,
    // G8a). G1/G2 omitted — their bytes are identical to chapter
    // 29's Q5/Q6.
    display_group_by_proofs(&fixture, platform_version);

    // Empirical probe of the value-tree element type for the two
    // single-property index terminators in the bench's contract
    // (`byBrand` is just `countable`, `byColor` is `rangeCountable`).
    // Surfaces the structural asymmetry that gates the
    // rangeCountable optimization.
    probe_value_tree_types(&fixture, platform_version);

    // Smoke test for grovedb PR #663's carrier-ACOR feature against
    // this bench's widget fixture. Exercises the proof shape that
    // would unblock chapter 30 G7 (`brand IN[...] AND color > floor`
    // with `group_by = [brand]`) at the grovedb layer, before drive
    // wires it through.
    probe_carrier_acor(&fixture, platform_version);

    // Outer-Range carrier-ACOR feasibility probe — the natural
    // extension of G7 from `outer In` to `outer Range`, with an
    // explicit SizedQuery limit on the outer walk. Drive doesn't
    // wire this through yet (mode_detection rejects 2 range clauses
    // up front); this probe is a feasibility check at the grovedb
    // layer.
    probe_carrier_acor_range_outer(&fixture, platform_version);

    let mut group = c.benchmark_group("document_count_worst_case");
    group.sample_size(10);
    group.throughput(criterion::Throughput::Elements(fixture.row_count));

    group.bench_function("group_by_in_proof_100_count_tree_branches", |b| {
        let raw_where = brand_in_where_value(brands.clone());
        b.iter_batched(
            || {
                count_request(
                    &fixture,
                    raw_where.clone(),
                    Value::Null,
                    CountMode::GroupByIn,
                    None,
                    true,
                )
            },
            |request| match fixture
                .drive
                .execute_document_count_request(request, None, platform_version)
                .expect("expected group_by In proof count request")
            {
                DocumentCountResponse::Proof(proof) => black_box(proof),
                response => panic!("expected proof response, got {response:?}"),
            },
            BatchSize::SmallInput,
        );
    });

    // Rangecountable-terminator variant of the In-grouped proof. The
    // contract's `byColor` index is `rangeCountable: true`, so the
    // covering value trees are themselves CountTrees and the
    // point-lookup builder skips the `[0]` descent (see
    // `point_lookup_count_path_query`'s "two terminator shapes"
    // section). Pairs with `group_by_in_proof_100_count_tree_branches`
    // (which targets the non-range_countable `byBrand` index) to
    // surface the optimization's per-branch byte savings.
    let colors = first_n_color_values(BRAND_COUNT);
    group.bench_function("group_by_color_in_proof_100_rangecountable_branches", |b| {
        let raw_where = color_in_where_value(colors.clone());
        b.iter_batched(
            || {
                count_request(
                    &fixture,
                    raw_where.clone(),
                    Value::Null,
                    CountMode::GroupByIn,
                    None,
                    true,
                )
            },
            |request| match fixture
                .drive
                .execute_document_count_request(request, None, platform_version)
                .expect("expected group_by color-In proof count request")
            {
                DocumentCountResponse::Proof(proof) => black_box(proof),
                response => panic!("expected proof response, got {response:?}"),
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("aggregate_in_range_no_proof_100_range_counts", |b| {
        let raw_where = in_and_range_where_value(brands.clone(), broad_range_floor.clone());
        b.iter_batched(
            || {
                count_request(
                    &fixture,
                    raw_where.clone(),
                    Value::Null,
                    CountMode::Aggregate,
                    None,
                    false,
                )
            },
            |request| match fixture
                .drive
                .execute_document_count_request(request, None, platform_version)
                .expect("expected aggregate In+range count request")
            {
                DocumentCountResponse::Aggregate(count) => black_box(count),
                response => panic!("expected aggregate response, got {response:?}"),
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("group_by_compound_in_range_no_proof_limit_100", |b| {
        let raw_where = in_and_range_where_value(brands.clone(), broad_range_floor.clone());
        b.iter_batched(
            || {
                count_request(
                    &fixture,
                    raw_where.clone(),
                    Value::Null,
                    CountMode::GroupByCompound,
                    Some(100),
                    false,
                )
            },
            |request| match fixture
                .drive
                .execute_document_count_request(request, None, platform_version)
                .expect("expected compound no-proof count request")
            {
                DocumentCountResponse::Entries(entries) => black_box(entries),
                response => panic!("expected entries response, got {response:?}"),
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("group_by_compound_in_range_proof_limit_100", |b| {
        let raw_where = in_and_range_where_value(brands.clone(), broad_range_floor.clone());
        b.iter_batched(
            || {
                count_request(
                    &fixture,
                    raw_where.clone(),
                    Value::Null,
                    CountMode::GroupByCompound,
                    Some(100),
                    true,
                )
            },
            |request| match fixture
                .drive
                .execute_document_count_request(request, None, platform_version)
                .expect("expected compound proof count request")
            {
                DocumentCountResponse::Proof(proof) => black_box(proof),
                response => panic!("expected proof response, got {response:?}"),
            },
            BatchSize::SmallInput,
        );
    });

    // Per-query timing for the 7 chapter queries (no group_by). Each
    // case exercises the same proof shape documented in
    // `book/src/drive/count-index-examples.md` so reviewers can quote
    // wall-clock timings alongside the proof-size and complexity
    // columns in the chapter's overview table.
    let mid_brand = brand_label(BRAND_COUNT / 2);
    let mid_color = color_label(color_count_for_rows(fixture.row_count) / 2);
    let brands_2 = brands_n(2);
    let colors_2 = first_n_color_values(2);
    let clause = |field: &str, op: &str, value: Value| -> Value {
        Value::Array(vec![
            Value::Text(field.to_string()),
            Value::Text(op.to_string()),
            value,
        ])
    };

    let chapter_queries: Vec<(&str, Value)> = vec![
        ("query_1_empty_total_count", Value::Null),
        (
            "query_2_brand_eq",
            Value::Array(vec![clause("brand", "==", Value::Text(mid_brand.clone()))]),
        ),
        (
            "query_3_color_eq",
            Value::Array(vec![clause("color", "==", Value::Text(mid_color.clone()))]),
        ),
        (
            "query_4_brand_eq_and_color_eq",
            Value::Array(vec![
                clause("brand", "==", Value::Text(mid_brand.clone())),
                clause("color", "==", Value::Text(mid_color.clone())),
            ]),
        ),
        (
            "query_5_brand_in_2",
            Value::Array(vec![clause("brand", "in", Value::Array(brands_2.clone()))]),
        ),
        (
            "query_6_color_in_2",
            Value::Array(vec![clause("color", "in", Value::Array(colors_2.clone()))]),
        ),
        (
            "query_7_color_gt_floor",
            Value::Array(vec![clause("color", ">", broad_range_floor.clone())]),
        ),
        (
            "query_8_brand_eq_and_color_gt_floor",
            Value::Array(vec![
                clause("brand", "==", Value::Text(mid_brand.clone())),
                clause("color", ">", broad_range_floor.clone()),
            ]),
        ),
    ];

    for (name, raw_where) in chapter_queries {
        group.bench_function(name, |b| {
            b.iter_batched(
                || {
                    count_request(
                        &fixture,
                        raw_where.clone(),
                        Value::Null,
                        CountMode::Aggregate,
                        None,
                        true,
                    )
                },
                |request| match fixture
                    .drive
                    .execute_document_count_request(request, None, platform_version)
                    .expect("expected proof response for chapter query")
                {
                    DocumentCountResponse::Proof(proof) => black_box(proof),
                    response => panic!("expected proof response, got {response:?}"),
                },
                BatchSize::SmallInput,
            );
        });
    }

    // Per-query timing for the Count Index Group By Examples chapter
    // (G1 through G1b plus G7/G8/G8a). Each case exercises one of
    // the documented group_by shapes so the chapter's overview
    // table can quote wall-clock timings alongside proof-size and
    // complexity columns.
    let brands_100 = brands_n(BRAND_COUNT);
    // Order-by-descending wire shape: matches what
    // `order_clauses_from_value` parses into a single
    // `OrderClause { field: brand, ascending: false }`. The
    // dispatcher reads the first order clause's direction to pick
    // `left_to_right` for the carrier walk on G8 / G8a.
    let order_by_brand_desc = Value::Array(vec![Value::Array(vec![
        Value::Text("brand".to_string()),
        Value::Text("desc".to_string()),
    ])]);
    let groupby_chapter_queries: Vec<(&str, Value, Value, CountMode, Option<u32>)> = vec![
        (
            "query_g1_brand_in_grouped_by_brand",
            Value::Array(vec![clause("brand", "in", Value::Array(brands_2.clone()))]),
            Value::Null,
            CountMode::GroupByIn,
            None,
        ),
        (
            // G1a: same `In on byBrand` shape as G1 but one of the
            // In values (`brand_100`) is absent from the fixture
            // (BRAND_COUNT = 100, so brand labels are
            // `brand_000`..`brand_099`). Captures the absent-branch
            // proof shape — the grovedb proof still commits an
            // absence subproof at the missing key, but
            // `verify_query` without
            // `absence_proofs_for_non_existing_searched_keys: true`
            // drops the absent branch from the returned entries
            // (see `test_point_lookup_proof_omits_absent_in_branches_from_entries`).
            "query_g1a_brand_in_with_absent_grouped_by_brand",
            Value::Array(vec![clause(
                "brand",
                "in",
                Value::Array(vec![
                    Value::Text(brand_label(0)),
                    Value::Text(brand_label(BRAND_COUNT)),
                ]),
            )]),
            Value::Null,
            CountMode::GroupByIn,
            None,
        ),
        (
            // G1b: same shape as G1, scaled to |IN| = BRAND_COUNT
            // = 100. The proof reveals every byBrand entry as a
            // `KVValueHashFeatureTypeWithChildHash` target — the
            // most efficient byte-per-key shape `GroupByIn` can
            // hit (no opaque-sibling commitments at L6).
            "query_g1b_brand_in_100_grouped_by_brand",
            Value::Array(vec![clause(
                "brand",
                "in",
                Value::Array(brands_100.clone()),
            )]),
            Value::Null,
            CountMode::GroupByIn,
            None,
        ),
        (
            "query_g2_color_in_grouped_by_color",
            Value::Array(vec![clause("color", "in", Value::Array(colors_2.clone()))]),
            Value::Null,
            CountMode::GroupByIn,
            None,
        ),
        (
            "query_g3_brand_in_color_eq_grouped_by_brand",
            Value::Array(vec![
                clause("brand", "in", Value::Array(brands_2.clone())),
                clause("color", "==", Value::Text(mid_color.clone())),
            ]),
            Value::Null,
            CountMode::GroupByIn,
            None,
        ),
        (
            "query_g4_color_gt_grouped_by_color",
            Value::Array(vec![clause("color", ">", broad_range_floor.clone())]),
            Value::Null,
            CountMode::GroupByRange,
            None,
        ),
        (
            "query_g5_brand_in_color_gt_grouped_by_brand_color",
            Value::Array(vec![
                clause("brand", "in", Value::Array(brands_2.clone())),
                clause("color", ">", broad_range_floor.clone()),
            ]),
            Value::Null,
            CountMode::GroupByCompound,
            None,
        ),
        (
            "query_g7_brand_in_color_gt_grouped_by_brand",
            Value::Array(vec![
                clause("brand", "in", Value::Array(brands_2.clone())),
                clause("color", ">", broad_range_floor.clone()),
            ]),
            Value::Null,
            CountMode::GroupByIn,
            None,
        ),
        (
            "query_g8_brand_gt_color_gt_grouped_by_brand",
            Value::Array(vec![
                clause("brand", ">", Value::Text(brand_label(BRAND_COUNT / 2))),
                clause("color", ">", broad_range_floor.clone()),
            ]),
            Value::Null,
            CountMode::GroupByRange,
            // Range-outer carrier-aggregate enforces a fixed
            // platform-wide outer-walk cap of
            // `MAX_CARRIER_AGGREGATE_OUTER_RANGE_LIMIT` (10); the
            // dispatcher rejects a caller-supplied `limit` on this
            // shape, so pass `None` here.
            None,
        ),
        (
            "query_g8a_brand_between_color_between_grouped_by_brand_desc",
            Value::Array(vec![
                // Two-sided brand range (brand_050, brand_065),
                // exclusive on both sides. The dispatcher merges
                // these into a single `BetweenExcludeBounds` clause
                // via `merge_same_field_range_pairs`.
                clause("brand", ">", Value::Text(brand_label(BRAND_COUNT / 2))),
                clause(
                    "brand",
                    "<",
                    Value::Text(brand_label(BRAND_COUNT * 65 / 100)),
                ),
                // Two-sided color range (color_00000200,
                // color_00000400), exclusive on both sides.
                clause("color", ">", Value::Text(color_label(200))),
                clause("color", "<", Value::Text(color_label(400))),
            ]),
            order_by_brand_desc.clone(),
            CountMode::GroupByRange,
            None,
        ),
    ];

    for (name, raw_where, raw_order_by, mode, limit) in groupby_chapter_queries {
        group.bench_function(name, |b| {
            b.iter_batched(
                || {
                    count_request(
                        &fixture,
                        raw_where.clone(),
                        raw_order_by.clone(),
                        mode,
                        limit,
                        true,
                    )
                },
                |request| match fixture
                    .drive
                    .execute_document_count_request(request, None, platform_version)
                    .expect("expected proof response for group_by chapter query")
                {
                    DocumentCountResponse::Proof(proof) => black_box(proof),
                    response => panic!("expected proof response, got {response:?}"),
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Run each proof-emitting shape once and print the resulting
/// `Vec<u8>` length. No timing — Criterion handles that — but byte
/// size is the actual win for the rangeCountable optimization, and
/// the only way to surface it from the same fixture without ad-hoc
/// instrumentation.
fn report_proof_sizes(
    fixture: &CountBenchFixture,
    brands: &[Value],
    broad_range_floor: &Value,
    platform_version: &PlatformVersion,
) {
    let colors_100 = first_n_color_values(BRAND_COUNT);
    let cases: [(&str, Value, Value, CountMode, Option<u32>); 3] = [
        // Non-rangeCountable `byBrand` In-grouped proof — control.
        (
            "group_by_in_proof_100_count_tree_branches",
            brand_in_where_value(brands.to_vec()),
            Value::Null,
            CountMode::GroupByIn,
            None,
        ),
        // RangeCountable `byColor` In-grouped proof — the shape the
        // optimization targets. Outer Keys resolve directly to the
        // value-tree CountTrees (no `[0]` descent), so this proof is
        // strictly smaller than the non-range_countable variant
        // above on the same fixture.
        (
            "group_by_color_in_proof_100_rangecountable_branches",
            color_in_where_value(colors_100),
            Value::Null,
            CountMode::GroupByIn,
            None,
        ),
        (
            "group_by_compound_in_range_proof_limit_100",
            in_and_range_where_value(brands.to_vec(), broad_range_floor.clone()),
            Value::Null,
            CountMode::GroupByCompound,
            Some(100),
        ),
    ];

    for (name, raw_where, raw_order_by, mode, limit) in cases {
        let request = count_request(fixture, raw_where, raw_order_by, mode, limit, true);
        match fixture
            .drive
            .execute_document_count_request(request, None, platform_version)
            .expect("expected proof response for proof-size report")
        {
            DocumentCountResponse::Proof(proof) => {
                eprintln!(
                    "[proof-size] rows={} {}: {} bytes",
                    fixture.row_count,
                    name,
                    proof.len()
                );
            }
            other => panic!("expected Proof response for {name}, got {other:?}"),
        }
    }
}

/// Run every `(group_by × where_shape)` combination of interest
/// through the drive count dispatcher and report whether each works
/// on the no-proof and prove paths.
///
/// **Drive vs. platform layer.** This is the drive-level dispatcher
/// (`Drive::execute_document_count_request`); the platform-level
/// handler (`drive-abci::query_documents_v1` →
/// `validate_and_route`) layers additional validation on top
/// (HAVING rejection; the `group_by` field-name vs `In`/range
/// where-clause alignment check; per-mode `limit` rejection).
/// Where the platform layer rejects a combination the drive layer
/// would technically accept, that's flagged in the `[matrix]`
/// output's annotations so the table the user sees reflects the
/// full request lifecycle.
///
/// Output is `[matrix] {key} = {result}` lines so callers can grep
/// them out of the bench's stderr stream.
fn report_group_by_matrix(fixture: &CountBenchFixture, platform_version: &PlatformVersion) {
    let brands_2: Vec<Value> = brands_n(2);
    let colors_2: Vec<Value> = first_n_color_values(2);
    let mid_brand = brand_label(BRAND_COUNT / 2);
    let mid_color = color_label(color_count_for_rows(fixture.row_count) / 2);
    let range_floor = Value::Text(fixture.range_floor.clone());

    // Compact builder for where-clause `Value::Array`s. Each inner
    // array is `[field, op, value]` — the wire shape the drive
    // dispatcher parses via `parse_count_where_value`.
    let clause = |field: &str, op: &str, value: Value| -> Value {
        Value::Array(vec![
            Value::Text(field.to_string()),
            Value::Text(op.to_string()),
            value,
        ])
    };
    let where_empty = || Value::Null;
    let where_brand_in =
        || Value::Array(vec![clause("brand", "in", Value::Array(brands_2.clone()))]);
    let where_color_in =
        || Value::Array(vec![clause("color", "in", Value::Array(colors_2.clone()))]);
    let where_brand_eq =
        || Value::Array(vec![clause("brand", "==", Value::Text(mid_brand.clone()))]);
    let where_color_eq =
        || Value::Array(vec![clause("color", "==", Value::Text(mid_color.clone()))]);
    let where_brand_eq_color_eq = || {
        Value::Array(vec![
            clause("brand", "==", Value::Text(mid_brand.clone())),
            clause("color", "==", Value::Text(mid_color.clone())),
        ])
    };
    let where_color_gt = || Value::Array(vec![clause("color", ">", range_floor.clone())]);
    let where_brand_in_color_gt = || {
        Value::Array(vec![
            clause("brand", "in", Value::Array(brands_2.clone())),
            clause("color", ">", range_floor.clone()),
        ])
    };
    let where_brand_in_color_eq = || {
        Value::Array(vec![
            clause("brand", "in", Value::Array(brands_2.clone())),
            clause("color", "==", Value::Text(mid_color.clone())),
        ])
    };
    let where_brand_eq_color_gt = || {
        Value::Array(vec![
            clause("brand", "==", Value::Text(mid_brand.clone())),
            clause("color", ">", range_floor.clone()),
        ])
    };
    let brand_floor = Value::Text(brand_label(BRAND_COUNT / 2));
    let where_brand_gt_color_gt = || {
        Value::Array(vec![
            clause("brand", ">", brand_floor.clone()),
            clause("color", ">", range_floor.clone()),
        ])
    };

    // (label, group_by-as-the-caller-would-spell-it, where description,
    //  raw where Value, CountMode used by drive, limit override,
    //  platform-allowed annotation).
    //
    // `platform_allowed` is the verdict from `validate_and_route` (the
    // platform-layer handler in `drive-abci`); annotated here from
    // direct reading of `dispatch_count_v1` since the bench can't
    // import drive-abci. Verified against the existing v1 handler
    // tests in `packages/rs-drive-abci/src/query/document_query/v1/tests.rs`
    // (the `reject_*` / `accept_*_routes_to_*` family).
    struct MatrixCase {
        label: &'static str,
        platform_allowed: &'static str,
        raw_where: Value,
        /// Order-by shape; `Value::Null` for the default-ascending
        /// path. Threaded through so order-sensitive carrier cases
        /// (G8a's descending walk) actually exercise
        /// `left_to_right = false` instead of silently defaulting
        /// to ascending.
        raw_order_by: Value,
        mode: CountMode,
        limit: Option<u32>,
    }

    let order_by_brand_desc = Value::Array(vec![Value::Array(vec![
        Value::Text("brand".to_string()),
        Value::Text("desc".to_string()),
    ])]);

    let cases: Vec<MatrixCase> = vec![
        // ── group_by = [] (Aggregate) ──────────────────────────────
        MatrixCase {
            label: "[] / where=(empty)",
            platform_allowed: "yes (documentsCountable fast path)",
            raw_where: where_empty(),
            raw_order_by: Value::Null,
            mode: CountMode::Aggregate,
            limit: None,
        },
        MatrixCase {
            label: "[] / where=brand==X",
            platform_allowed: "yes",
            raw_where: where_brand_eq(),
            raw_order_by: Value::Null,
            mode: CountMode::Aggregate,
            limit: None,
        },
        MatrixCase {
            label: "[] / where=color==X",
            platform_allowed: "yes",
            raw_where: where_color_eq(),
            raw_order_by: Value::Null,
            mode: CountMode::Aggregate,
            limit: None,
        },
        MatrixCase {
            label: "[] / where=brand==X AND color==Y",
            platform_allowed: "yes",
            raw_where: where_brand_eq_color_eq(),
            raw_order_by: Value::Null,
            mode: CountMode::Aggregate,
            limit: None,
        },
        MatrixCase {
            label: "[] / where=brand IN[2]",
            platform_allowed: "yes (per-In aggregate fan-out)",
            raw_where: where_brand_in(),
            raw_order_by: Value::Null,
            mode: CountMode::Aggregate,
            limit: None,
        },
        MatrixCase {
            label: "[] / where=color IN[2]",
            platform_allowed: "yes (per-In aggregate fan-out)",
            raw_where: where_color_in(),
            raw_order_by: Value::Null,
            mode: CountMode::Aggregate,
            limit: None,
        },
        MatrixCase {
            label: "[] / where=color > floor",
            platform_allowed: "yes (AggregateCountOnRange)",
            raw_where: where_color_gt(),
            raw_order_by: Value::Null,
            mode: CountMode::Aggregate,
            limit: None,
        },
        MatrixCase {
            label: "[] / where=brand==X AND color > floor",
            platform_allowed: "yes (AggregateCountOnRange on byBrandColor terminator)",
            raw_where: where_brand_eq_color_gt(),
            raw_order_by: Value::Null,
            mode: CountMode::Aggregate,
            limit: None,
        },
        MatrixCase {
            label: "[] / where=brand IN[2] AND color > floor",
            platform_allowed: "no-proof: yes / prove: no (aggregate proof can't fork)",
            raw_where: where_brand_in_color_gt(),
            raw_order_by: Value::Null,
            mode: CountMode::Aggregate,
            limit: None,
        },
        // G8c: same `where` as G8, but with no group_by. The
        // two-range carrier requires `GroupByRange + group_by =
        // [outer_range_field]`; with `mode = Aggregate` the
        // dispatcher rejects at mode-detection (single-`u64`
        // aggregation across two ranges has no defined target —
        // the per-branch counts can't be silently summed at the
        // verifier).
        MatrixCase {
            label: "[] / where=brand > floor AND color > floor",
            platform_allowed:
                "no — two-range carrier requires `GroupByRange + group_by = [outer_range_field]`",
            raw_where: where_brand_gt_color_gt(),
            raw_order_by: Value::Null,
            mode: CountMode::Aggregate,
            limit: None,
        },
        // ── group_by = [color] (single-field) ──────────────────────
        MatrixCase {
            label: "[color] / where=color IN[2]",
            platform_allowed: "yes (GroupByIn)",
            raw_where: where_color_in(),
            raw_order_by: Value::Null,
            mode: CountMode::GroupByIn,
            limit: None,
        },
        MatrixCase {
            label: "[color] / where=color > floor",
            platform_allowed: "yes (GroupByRange — distinct-range walk)",
            raw_where: where_color_gt(),
            raw_order_by: Value::Null,
            mode: CountMode::GroupByRange,
            limit: None,
        },
        MatrixCase {
            label: "[color] / where=color==X",
            platform_allowed: "no — `color` is constrained by `==`, not `In` or range",
            raw_where: where_color_eq(),
            raw_order_by: Value::Null,
            mode: CountMode::GroupByIn,
            limit: None,
        },
        MatrixCase {
            label: "[color] / where=brand IN[2] AND color > floor",
            platform_allowed: "no — single-field GROUP BY with both `In` and range",
            raw_where: where_brand_in_color_gt(),
            raw_order_by: Value::Null,
            mode: CountMode::GroupByRange,
            limit: None,
        },
        // ── group_by = [brand] (single-field) ──────────────────────
        MatrixCase {
            label: "[brand] / where=brand IN[2]",
            platform_allowed: "yes (GroupByIn — non-rangeCountable byBrand)",
            raw_where: where_brand_in(),
            raw_order_by: Value::Null,
            mode: CountMode::GroupByIn,
            limit: None,
        },
        MatrixCase {
            label: "[brand] / where=brand IN[2] AND color==Y",
            platform_allowed: "yes (GroupByIn — compound covers byBrandColor)",
            raw_where: where_brand_in_color_eq(),
            raw_order_by: Value::Null,
            mode: CountMode::GroupByIn,
            limit: None,
        },
        MatrixCase {
            label: "[brand] / where=brand IN[2] AND color > floor",
            platform_allowed: "yes (RangeAggregateCarrierProof — carrier ACOR per In branch)",
            raw_where: where_brand_in_color_gt(),
            raw_order_by: Value::Null,
            mode: CountMode::GroupByIn,
            limit: None,
        },
        MatrixCase {
            label: "[brand] / where=brand > floor AND color > floor",
            platform_allowed:
                "yes (RangeAggregateCarrierProof — carrier ACOR; platform-max outer limit = 10)",
            raw_where: where_brand_gt_color_gt(),
            raw_order_by: Value::Null,
            mode: CountMode::GroupByRange,
            limit: None,
        },
        // G8a: bounded carrier + bounded ACOR with descending walk.
        // Same `RangeAggregateCarrierProof` mode as G8 but the
        // dispatcher merges two-sided ranges into `between*` clauses
        // via `merge_same_field_range_pairs` and threads
        // `left_to_right = false` through the carrier path query.
        MatrixCase {
            label: "[brand] / where=brand BETWEEN AND color BETWEEN (left_to_right=false)",
            platform_allowed:
                "yes (RangeAggregateCarrierProof — bounded-range carrier with descending walk)",
            raw_where: Value::Array(vec![
                clause("brand", ">", Value::Text(brand_label(BRAND_COUNT / 2))),
                clause(
                    "brand",
                    "<",
                    Value::Text(brand_label(BRAND_COUNT * 65 / 100)),
                ),
                clause("color", ">", Value::Text(color_label(200))),
                clause("color", "<", Value::Text(color_label(400))),
            ]),
            raw_order_by: order_by_brand_desc.clone(),
            mode: CountMode::GroupByRange,
            limit: None,
        },
        MatrixCase {
            label: "[brand] / where=brand==X",
            platform_allowed: "no — `brand` is `==`, not `In` or range",
            raw_where: where_brand_eq(),
            raw_order_by: Value::Null,
            mode: CountMode::GroupByIn,
            limit: None,
        },
        // ── group_by = [brand, color] (two-field compound) ─────────
        MatrixCase {
            label: "[brand, color] / where=brand IN[2] AND color > floor",
            platform_allowed: "yes (GroupByCompound — `(In, range)` shape)",
            raw_where: where_brand_in_color_gt(),
            raw_order_by: Value::Null,
            mode: CountMode::GroupByCompound,
            limit: Some(100),
        },
        MatrixCase {
            label: "[brand, color] / where=brand IN[2] AND color==Y",
            platform_allowed: "no — `color` must be range, not `==`",
            raw_where: where_brand_in_color_eq(),
            raw_order_by: Value::Null,
            mode: CountMode::GroupByCompound,
            limit: Some(100),
        },
        // G8b: same `where` as G8, but group_by widened to
        // [brand, color]. The two-range carrier (`brand > floor AND
        // color > floor`) is permitted only with
        // `GroupByRange + group_by = [outer_range_field]`; with
        // `GroupByCompound + group_by = [outer, inner]` the
        // dispatcher rejects at mode-detection (the carrier shape
        // is single-field only — the compound walk would need a
        // distinct enumeration over both ranges, which the carrier
        // primitive doesn't express).
        MatrixCase {
            label: "[brand, color] / where=brand > floor AND color > floor",
            platform_allowed:
                "no — two-range carrier requires `GroupByRange + group_by = [outer_range_field]`",
            raw_where: where_brand_gt_color_gt(),
            raw_order_by: Value::Null,
            mode: CountMode::GroupByCompound,
            limit: None,
        },
        // ── group_by = [color, brand] (reversed compound) ──────────
        MatrixCase {
            label: "[color, brand] / where=color IN[2] AND brand > X",
            platform_allowed: "no — no rangeCountable index has `brand` as terminator",
            // brand > X would need a covering rangeCountable index
            // with brand as the terminator. The contract has none, so
            // the picker errors at drive level too.
            raw_where: Value::Array(vec![
                clause("color", "in", Value::Array(colors_2.clone())),
                clause("brand", ">", Value::Text(mid_brand.clone())),
            ]),
            raw_order_by: Value::Null,
            mode: CountMode::GroupByCompound,
            limit: Some(100),
        },
    ];

    for case in &cases {
        let noproof_result = drive_count_outcome(
            fixture,
            case.raw_where.clone(),
            case.raw_order_by.clone(),
            case.mode,
            case.limit,
            false,
            platform_version,
        );
        let prove_result = drive_count_outcome(
            fixture,
            case.raw_where.clone(),
            case.raw_order_by.clone(),
            case.mode,
            case.limit,
            true,
            platform_version,
        );
        eprintln!(
            "[matrix] {label}\n         no-proof: {np}\n         prove:    {pr}\n         platform: {pa}",
            label = case.label,
            np = noproof_result,
            pr = prove_result,
            pa = case.platform_allowed,
        );
    }
}

/// Dump the actual grovedb proof bytes (hex) for every
/// `group_by = []` (Aggregate) prove case. Each proof's byte layout
/// is determined by which grovedb primitive the drive dispatcher
/// routes to:
///
/// - `(empty)` → primary-key CountTree proof at the doctype's `[0]`
///   child (`documentsCountable: true` fast path; the proof is
///   merk-path to a single CountTree element).
/// - `field == X` → `point_lookup_count_path_query` against the
///   covering countable index; the proof is a merk-path to either
///   `[..., last_value, 0]` (normal countable) or `[..., last_value]`
///   (rangeCountable, post-optimization).
/// - `field IN [...]` → same `point_lookup_count_path_query` shape
///   but with one outer `Key` per In value, so the proof carries
///   one merk-path per resolved branch.
/// - `range_field > floor` → `aggregate_count_path_query` against
///   the rangeCountable terminator's property-name `ProvableCountTree`;
///   the proof is an `AggregateCountOnRange` primitive that signs
///   a single u64.
///
/// Hex is emitted 64 hex chars per line (32 bytes per row) so the
/// output is grep-able and the rows align with merk-tree node
/// boundaries on most layouts.
/// Probe what's *actually* stored at `widget/brand/brand_050` and at
/// `widget/color/color_00000500` so a reviewer can confirm by reading
/// the live fixture which element types the two indexes produce.
///
/// This is the empirical answer to "why can't `byBrand` use the same
/// `path=[..., "brand"], Key("brand_050")` shape as `byColor`?". The
/// shape only works when the resolved element is itself a count-bearing
/// tree — for byBrand (just `countable`, not `rangeCountable`) the
/// value tree is `Element::Tree` (a `NormalTree`), and
/// `NormalTree::count_value_or_default()` returns `1`, not the doc
/// count. The optimization is structurally gated on the index's
/// `range_countable` flag for this exact reason.
fn probe_value_tree_types(fixture: &CountBenchFixture, _platform_version: &PlatformVersion) {
    use drive::drive::RootTree;
    use grovedb_path::SubtreePath;

    let contract_id = fixture.data_contract.id().to_buffer();
    let cases: [(&'static str, &'static str, &'static str); 2] = [
        ("byBrand", "brand", "brand_050"),
        ("byColor", "color", "color_00000500"),
    ];
    let grove_version = &PlatformVersion::latest().drive.grove_version;

    for (label, prop, val) in cases {
        let parent: Vec<&[u8]> = vec![
            &[RootTree::DataContractDocuments as u8],
            &contract_id,
            &[1u8],
            DOCUMENT_TYPE_NAME.as_bytes(),
            prop.as_bytes(),
        ];
        let key = val.as_bytes();
        match fixture
            .drive
            .grove
            .get(SubtreePath::from(parent.as_slice()), key, None, grove_version)
            .unwrap()
        {
            Ok(elem) => eprintln!(
                "[probe] {label}: widget/{prop}/{val} → {} {{ count_value_or_default: {}, debug: {:?} }}",
                element_variant_name(&elem),
                elem.count_value_or_default(),
                elem
            ),
            Err(e) => eprintln!("[probe] {label}: widget/{prop}/{val} → grove.get error: {e:?}"),
        }
    }

    // Probe the CHILDREN of each value tree to see how each one
    // contributes to the parent's count_value_or_default. The
    // byBrand value tree has children:
    //   - `[0]` (the ref-bucket CountTree where byBrand's
    //     references live)
    //   - `color` (the byBrandColor continuation's property-name
    //     tree)
    // Are either of them wrapped in `Element::NonCounted(_)`? That
    // determines whether a hypothetical "value tree is always a
    // CountTree" rule would yield the correct count.
    let child_probes: [(&'static str, &'static str, &'static str, &'static [u8]); 4] = [
        ("byBrand /[0] ref-bucket", "brand", "brand_050", &[0u8]),
        (
            "byBrand /color continuation",
            "brand",
            "brand_050",
            b"color",
        ),
        ("byColor /[0] ref-bucket", "color", "color_00000500", &[0u8]),
        (
            "byColor /brand continuation",
            "color",
            "color_00000500",
            b"brand",
        ),
    ];
    for (label, prop, val, child) in child_probes {
        let parent_owned: Vec<Vec<u8>> = vec![
            vec![RootTree::DataContractDocuments as u8],
            contract_id.to_vec(),
            vec![1u8],
            DOCUMENT_TYPE_NAME.as_bytes().to_vec(),
            prop.as_bytes().to_vec(),
            val.as_bytes().to_vec(),
        ];
        let parent_refs: Vec<&[u8]> = parent_owned.iter().map(|v| v.as_slice()).collect();
        match fixture
            .drive
            .grove
            .get(
                SubtreePath::from(parent_refs.as_slice()),
                child,
                None,
                grove_version,
            )
            .unwrap()
        {
            Ok(elem) => eprintln!(
                "[probe-child] {label} (child_key={}): {} {{ count_value_or_default: {}, debug: {:?} }}",
                display_segment(child),
                element_variant_name(&elem),
                elem.count_value_or_default(),
                elem
            ),
            Err(e) => eprintln!(
                "[probe-child] {label} (child_key={}): grove.get error: {e:?}",
                display_segment(child)
            ),
        }
    }
}

/// Smoke test for the carrier-ACOR feature shipped in
/// [grovedb PR #663](https://github.com/dashpay/grovedb/pull/663).
///
/// Exercises the new `Query::set_subquery(Query::new_aggregate_count_on_range(...))`
/// composition against this bench's widget fixture: builds a `PathQuery` rooted
/// at `widget/brand` with two outer `In` keys (brand_000 + brand_001) and an
/// `AggregateCountOnRange` subquery on each brand's `color` subtree
/// (`color > "color_00000500"`).
///
/// This is the proof shape that would unblock chapter 30 G7 — `brand IN[...] AND
/// color > floor` grouped by `[brand]` — once drive wires it through. The probe
/// runs three separate operations against grovedb to confirm round-trip parity:
///
/// 1. **No-proof:** `query_aggregate_count_per_key` reads the raw counts.
/// 2. **Prove:** `prove_query` emits the carrier proof bytes.
/// 3. **Verify:** `verify_aggregate_count_query_per_key` reconstructs the
///    counts from the proof and confirms the root hash matches the parent
///    grovedb state.
///
/// Expected payload for this fixture (1 doc per `(brand, color)` pair, 1 000
/// colors per brand, range `color > "color_00000500"`):
///
/// ```text
/// [("brand_000", 499), ("brand_001", 499)]
/// ```
///
/// Printed under `[carrier-acor]` so reviewers can grep deterministically.
fn probe_carrier_acor(fixture: &CountBenchFixture, platform_version: &PlatformVersion) {
    use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
    use drive::drive::RootTree;
    use grovedb::{Query, QueryItem, SizedQuery};

    let grove_version = &platform_version.drive.grove_version;
    let contract_id = fixture.data_contract.id().to_buffer();
    let document_type = fixture
        .data_contract
        .document_type_for_name(DOCUMENT_TYPE_NAME)
        .expect("widget doc type");

    // Serialize the In keys (brand_000, brand_001) the same way drive's
    // index machinery would so the keys round-trip against the on-disk
    // byBrand subtree.
    let brand_keys: Vec<Vec<u8>> = (0..2)
        .map(|i| {
            document_type
                .serialize_value_for_key("brand", &Value::Text(brand_label(i)), platform_version)
                .expect("expected to serialize brand")
        })
        .collect();

    // Serialize the range floor (color_00000500) for the inner ACOR item.
    let range_floor_key = document_type
        .serialize_value_for_key(
            "color",
            &Value::Text(fixture.range_floor.clone()),
            platform_version,
        )
        .expect("expected to serialize range floor");

    // Build the carrier query — outer Keys for the brands, subquery_path
    // descending into each brand's `color` subtree, subquery as the
    // ACOR over `color > range_floor`. Insert via `insert_key` so the
    // multi-key walker sees the keys in lex-ascending order (grovedb
    // PR #663's invariant).
    let mut carrier: Query = Query::new();
    for k in &brand_keys {
        carrier.insert_key(k.clone());
    }
    carrier.set_subquery_path(vec![b"color".to_vec()]);
    carrier.set_subquery(Query::new_aggregate_count_on_range(QueryItem::RangeAfter(
        range_floor_key..,
    )));

    let path: Vec<Vec<u8>> = vec![
        vec![RootTree::DataContractDocuments as u8],
        contract_id.to_vec(),
        vec![1u8],
        DOCUMENT_TYPE_NAME.as_bytes().to_vec(),
        b"brand".to_vec(),
    ];
    let path_query = PathQuery::new(path, SizedQuery::new(carrier, None, None));

    eprintln!(
        "[carrier-acor] probing: widget/brand IN [brand_000, brand_001] subquery_path=color subquery=AggregateCountOnRange(RangeAfter(color_00000500..))"
    );

    // 1. No-proof: raw query.
    match fixture
        .drive
        .grove
        .query_aggregate_count_per_key(&path_query, None, grove_version)
        .unwrap()
    {
        Ok(entries) => {
            eprintln!("[carrier-acor] no-proof entries ({}):", entries.len());
            for (k, c) in &entries {
                eprintln!("[carrier-acor]   ({}, {})", display_segment(k), c);
            }
        }
        Err(e) => eprintln!("[carrier-acor] no-proof error: {e:?}"),
    }

    // 2. Prove: get the carrier-ACOR proof bytes.
    let proof = match fixture
        .drive
        .grove
        .prove_query(&path_query, None, grove_version)
        .unwrap()
    {
        Ok(p) => {
            eprintln!("[carrier-acor] proof bytes: {} B", p.len());
            p
        }
        Err(e) => {
            eprintln!("[carrier-acor] prove_query error: {e:?}");
            return;
        }
    };

    // 3. Verify the proof and confirm we get the same per-key counts back.
    match GroveDb::verify_aggregate_count_query_per_key(&proof, &path_query, grove_version) {
        Ok((root, entries)) => {
            eprintln!("[carrier-acor] verified root_hash: 0x{}", hex_bytes(&root));
            eprintln!("[carrier-acor] verified entries ({}):", entries.len());
            for (k, c) in &entries {
                eprintln!("[carrier-acor]   ({}, {})", display_segment(k), c);
            }
        }
        Err(e) => eprintln!("[carrier-acor] verify error: {e:?}"),
    }
}

/// Companion to [`probe_carrier_acor`] that exercises the
/// *outer-Range* variant of grovedb's carrier-ACOR feature
/// ([PR #663](https://github.com/dashpay/grovedb/pull/663)'s
/// `validate_carrier_aggregate_count_accepts_range_outer_items`).
///
/// Constructs a carrier PathQuery whose outer dimension walks a
/// **range** of In-property values (brand `> "brand_050"`) capped
/// at 20 results, with the same per-brand ACOR subquery over
/// `color > "color_00000500"`. Prints the per-brand aggregate
/// counts under `[carrier-acor-range]` so reviewers can grep
/// deterministically.
///
/// Expected output for this fixture (1 doc per `(brand, color)`
/// pair, 100 brands, 1 000 colors per brand, limit 20):
/// 20 entries for `brand_051` … `brand_070`, each carrying
/// `count = 499` (every brand has 499 colors `> "color_00000500"`).
///
/// This is the proof shape that would unblock "Q8 with a range
/// outer + ACOR inner, limit 20" — the natural extension of G7
/// from `outer In` to `outer Range`. Drive doesn't wire this
/// through yet; this probe is a feasibility check against the
/// existing grovedb plumbing.
fn probe_carrier_acor_range_outer(fixture: &CountBenchFixture, platform_version: &PlatformVersion) {
    use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
    use drive::drive::RootTree;
    use grovedb::{Query, QueryItem, SizedQuery};

    let grove_version = &platform_version.drive.grove_version;
    let contract_id = fixture.data_contract.id().to_buffer();
    let document_type = fixture
        .data_contract
        .document_type_for_name(DOCUMENT_TYPE_NAME)
        .expect("widget doc type");

    // Serialize the range floor for the OUTER dimension (brand > "brand_050").
    let brand_floor_key = document_type
        .serialize_value_for_key(
            "brand",
            &Value::Text(brand_label(BRAND_COUNT / 2)),
            platform_version,
        )
        .expect("expected to serialize outer brand floor");
    // Serialize the range floor for the INNER ACOR (color > "color_00000500").
    let color_floor_key = document_type
        .serialize_value_for_key(
            "color",
            &Value::Text(fixture.range_floor.clone()),
            platform_version,
        )
        .expect("expected to serialize inner color floor");

    let mut carrier: Query = Query::new();
    carrier
        .items
        .push(QueryItem::RangeAfter(brand_floor_key.clone()..));
    carrier.set_subquery_path(vec![b"color".to_vec()]);
    carrier.set_subquery(Query::new_aggregate_count_on_range(QueryItem::RangeAfter(
        color_floor_key..,
    )));

    let path: Vec<Vec<u8>> = vec![
        vec![RootTree::DataContractDocuments as u8],
        contract_id.to_vec(),
        vec![1u8],
        DOCUMENT_TYPE_NAME.as_bytes().to_vec(),
        b"brand".to_vec(),
    ];
    // `SizedQuery::limit` on carrier-ACOR is now permitted per
    // [grovedb PR #664](https://github.com/dashpay/grovedb/pull/664)
    // (the follow-up to PR #663 that split the leaf-strict vs
    // carrier-permissive validators on `SizedQuery::limit` /
    // `SizedQuery::offset`). The limit caps the number of outer-key
    // matches the carrier walks — each matched outer key still
    // produces a complete leaf-ACOR `u64`. The probe matches the
    // platform-wide cap defined at
    // `MAX_CARRIER_AGGREGATE_OUTER_RANGE_LIMIT` (10), which the drive
    // dispatcher enforces on the G8 shape.
    let outer_limit: u16 = 10;
    let path_query = PathQuery::new(path, SizedQuery::new(carrier, Some(outer_limit), None));

    eprintln!(
        "[carrier-acor-range] probing: widget/brand RangeAfter(brand_050..) limit={outer_limit} \
         subquery_path=color subquery=AggregateCountOnRange(RangeAfter(color_00000500..))"
    );

    // 1. No-proof.
    match fixture
        .drive
        .grove
        .query_aggregate_count_per_key(&path_query, None, grove_version)
        .unwrap()
    {
        Ok(entries) => {
            eprintln!("[carrier-acor-range] no-proof entries ({}):", entries.len());
            for (k, c) in &entries {
                eprintln!("[carrier-acor-range]   ({}, {})", display_segment(k), c);
            }
        }
        Err(e) => eprintln!("[carrier-acor-range] no-proof error: {e:?}"),
    }

    // 2. Prove.
    let proof = match fixture
        .drive
        .grove
        .prove_query(&path_query, None, grove_version)
        .unwrap()
    {
        Ok(p) => {
            eprintln!("[carrier-acor-range] proof bytes: {} B", p.len());
            p
        }
        Err(e) => {
            eprintln!("[carrier-acor-range] prove_query error: {e:?}");
            return;
        }
    };

    // 3. Verify.
    match GroveDb::verify_aggregate_count_query_per_key(&proof, &path_query, grove_version) {
        Ok((root, entries)) => {
            eprintln!(
                "[carrier-acor-range] verified root_hash: 0x{}",
                hex_bytes(&root)
            );
            eprintln!("[carrier-acor-range] verified entries ({}):", entries.len());
            for (k, c) in &entries {
                eprintln!("[carrier-acor-range]   ({}, {})", display_segment(k), c);
            }
        }
        Err(e) => eprintln!("[carrier-acor-range] verify error: {e:?}"),
    }
}

fn element_variant_name(e: &grovedb::Element) -> &'static str {
    use grovedb::Element;
    match e {
        Element::CountTree(_, _, _) => "CountTree",
        Element::ProvableCountTree(_, _, _) => "ProvableCountTree",
        Element::SumTree(_, _, _) => "SumTree",
        Element::CountSumTree(_, _, _, _) => "CountSumTree",
        Element::ProvableCountSumTree(_, _, _, _) => "ProvableCountSumTree",
        Element::Tree(_, _) => "Tree (NormalTree)",
        Element::Item(_, _) => "Item",
        Element::Reference(_, _, _) => "Reference",
        _ => "(other-variant)",
    }
}

/// Decoded display of every `group_by = []` proof shape.
///
/// For each case, this:
/// 1. Re-runs the drive dispatcher to get the proof bytes.
/// 2. Reconstructs the **same `PathQuery`** the prover used (by
///    calling the matching builder on `DriveDocumentCountQuery` —
///    the single source of truth shared by prover + verifier).
/// 3. Runs the appropriate grovedb verifier
///    (`verify_query` for point-lookup / primary-key proofs,
///    `verify_aggregate_count_query` for the range-aggregate
///    primitive) and prints the verified payload.
///
/// The output is structured so a reader can correlate each
/// proof's size with the path-query shape AND the merk elements
/// the proof signs, without parsing raw merk-proof bytes.
fn display_proofs(fixture: &CountBenchFixture, platform_version: &PlatformVersion) {
    let document_type = fixture
        .data_contract
        .document_type_for_name(DOCUMENT_TYPE_NAME)
        .expect("widget doc type");
    let contract_id = fixture.data_contract.id().to_buffer();
    let brands_2 = brands_n(2);
    let colors_2 = first_n_color_values(2);
    let mid_brand = brand_label(BRAND_COUNT / 2);
    let mid_color = color_label(color_count_for_rows(fixture.row_count) / 2);
    let range_floor = Value::Text(fixture.range_floor.clone());

    // Helper: wire-shaped where Value the dispatcher CBOR-decodes.
    let clause = |field: &str, op: &str, value: Value| -> Value {
        Value::Array(vec![
            Value::Text(field.to_string()),
            Value::Text(op.to_string()),
            value,
        ])
    };

    // Each case carries:
    // - `label`: how it appears in the table
    // - `raw_where`: wire-shaped where value passed to the dispatcher
    // - `structured`: structured WhereClauses the verifier-side path
    //   query builder consumes (mirrors what `parse_count_where_value`
    //   would produce on the dispatcher side)
    // - `shape`: which verifier primitive applies
    enum Shape {
        PrimaryKey,
        PointLookup,
        AggregateRange,
    }

    struct DisplayCase {
        label: &'static str,
        raw_where: Value,
        structured: Vec<WhereClause>,
        shape: Shape,
    }

    let cases: Vec<DisplayCase> = vec![
        DisplayCase {
            label: "[] / where=(empty)",
            raw_where: Value::Null,
            structured: vec![],
            shape: Shape::PrimaryKey,
        },
        DisplayCase {
            label: "[] / where=brand==X",
            raw_where: Value::Array(vec![clause("brand", "==", Value::Text(mid_brand.clone()))]),
            structured: vec![WhereClause {
                field: "brand".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text(mid_brand.clone()),
            }],
            shape: Shape::PointLookup,
        },
        DisplayCase {
            label: "[] / where=color==X",
            raw_where: Value::Array(vec![clause("color", "==", Value::Text(mid_color.clone()))]),
            structured: vec![WhereClause {
                field: "color".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text(mid_color.clone()),
            }],
            shape: Shape::PointLookup,
        },
        DisplayCase {
            label: "[] / where=brand==X AND color==Y",
            raw_where: Value::Array(vec![
                clause("brand", "==", Value::Text(mid_brand.clone())),
                clause("color", "==", Value::Text(mid_color.clone())),
            ]),
            structured: vec![
                WhereClause {
                    field: "brand".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Text(mid_brand.clone()),
                },
                WhereClause {
                    field: "color".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Text(mid_color.clone()),
                },
            ],
            shape: Shape::PointLookup,
        },
        DisplayCase {
            label: "[] / where=brand IN[2]",
            raw_where: Value::Array(vec![clause("brand", "in", Value::Array(brands_2.clone()))]),
            structured: vec![WhereClause {
                field: "brand".to_string(),
                operator: WhereOperator::In,
                value: Value::Array(brands_2.clone()),
            }],
            shape: Shape::PointLookup,
        },
        DisplayCase {
            label: "[] / where=color IN[2]",
            raw_where: Value::Array(vec![clause("color", "in", Value::Array(colors_2.clone()))]),
            structured: vec![WhereClause {
                field: "color".to_string(),
                operator: WhereOperator::In,
                value: Value::Array(colors_2.clone()),
            }],
            shape: Shape::PointLookup,
        },
        DisplayCase {
            label: "[] / where=color > floor",
            raw_where: Value::Array(vec![clause("color", ">", range_floor.clone())]),
            structured: vec![WhereClause {
                field: "color".to_string(),
                operator: WhereOperator::GreaterThan,
                value: range_floor.clone(),
            }],
            shape: Shape::AggregateRange,
        },
        DisplayCase {
            label: "[] / where=brand==X AND color > floor",
            raw_where: Value::Array(vec![
                clause("brand", "==", Value::Text(mid_brand.clone())),
                clause("color", ">", range_floor.clone()),
            ]),
            structured: vec![
                WhereClause {
                    field: "brand".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Text(mid_brand.clone()),
                },
                WhereClause {
                    field: "color".to_string(),
                    operator: WhereOperator::GreaterThan,
                    value: range_floor.clone(),
                },
            ],
            shape: Shape::AggregateRange,
        },
    ];

    for case in cases {
        // 1. Get proof bytes via the drive dispatcher (the same code
        //    path the bench measures).
        let request = count_request(
            fixture,
            case.raw_where,
            Value::Null,
            CountMode::Aggregate,
            None,
            true,
        );
        let proof =
            match fixture
                .drive
                .execute_document_count_request(request, None, platform_version)
            {
                Ok(DocumentCountResponse::Proof(p)) => p,
                other => {
                    eprintln!(
                        "[proof] {label} → unexpected non-Proof response: {other:?}",
                        label = case.label
                    );
                    continue;
                }
            };

        // 2. Reconstruct the path query the prover used so we can
        //    verify with the same spec.
        let path_query: PathQuery = match case.shape {
            Shape::PrimaryKey => DriveDocumentCountQuery::primary_key_count_tree_path_query(
                contract_id,
                DOCUMENT_TYPE_NAME,
            ),
            Shape::PointLookup => {
                let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
                    document_type.indexes(),
                    &case.structured,
                    &[],
                )
                .expect("countable picker must find a covering index for the display case");
                let query = DriveDocumentCountQuery {
                    document_type,
                    contract_id,
                    document_type_name: DOCUMENT_TYPE_NAME.to_string(),
                    index,
                    where_clauses: case.structured.clone(),
                };
                query
                    .point_lookup_count_path_query(platform_version)
                    .expect("point-lookup builder must accept the display case's shape")
            }
            Shape::AggregateRange => {
                let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
                    document_type.indexes(),
                    &case.structured,
                    &[],
                )
                .expect("range_countable picker must find a covering index");
                let query = DriveDocumentCountQuery {
                    document_type,
                    contract_id,
                    document_type_name: DOCUMENT_TYPE_NAME.to_string(),
                    index,
                    where_clauses: case.structured.clone(),
                };
                query
                    .aggregate_count_path_query(platform_version)
                    .expect("aggregate-range builder must accept the display case's shape")
            }
        };

        eprintln!(
            "[proof] {label} ({sz} bytes)",
            label = case.label,
            sz = proof.len()
        );

        // 3. Print the path-query spec.
        eprintln!("[proof]   path:");
        for seg in &path_query.path {
            eprintln!("[proof]     {}", display_segment(seg));
        }
        eprintln!(
            "[proof]   query items: {}",
            display_query_items(&path_query.query.query.items)
        );
        let sb = &path_query.query.query.default_subquery_branch;
        if let Some(sqp) = sb.subquery_path.as_ref() {
            let pretty: Vec<String> = sqp.iter().map(|s| display_segment(s)).collect();
            eprintln!("[proof]   subquery_path: [{}]", pretty.join(", "));
        }
        if let Some(sq) = sb.subquery.as_ref() {
            eprintln!(
                "[proof]   subquery items: {}",
                display_query_items(&sq.items)
            );
        }

        // 4. Verify + print the structured payload.
        match case.shape {
            Shape::AggregateRange => {
                match GroveDb::verify_aggregate_count_query(
                    &proof,
                    &path_query,
                    &platform_version.drive.grove_version,
                ) {
                    Ok((root, count)) => {
                        eprintln!("[proof]   verified:");
                        eprintln!("[proof]     root_hash: 0x{}", hex_bytes(&root));
                        eprintln!("[proof]     count: {count}");
                    }
                    Err(e) => eprintln!("[proof]   verify error: {e:?}"),
                }
            }
            Shape::PrimaryKey | Shape::PointLookup => {
                match GroveDb::verify_query(
                    &proof,
                    &path_query,
                    &platform_version.drive.grove_version,
                ) {
                    Ok((root, elements)) => {
                        eprintln!("[proof]   verified:");
                        eprintln!("[proof]     root_hash: 0x{}", hex_bytes(&root));
                        eprintln!("[proof]     elements ({}):", elements.len());
                        for (path, key, elem) in elements {
                            let path_pretty: Vec<String> =
                                path.iter().map(|s| display_segment(s)).collect();
                            eprintln!("[proof]       path: [{}]", path_pretty.join(", "));
                            eprintln!("[proof]       key:  {}", display_segment(&key));
                            eprintln!("[proof]       element: {}", display_element(elem.as_ref()));
                        }
                    }
                    Err(e) => eprintln!("[proof]   verify error: {e:?}"),
                }
            }
        }

        // 5. Decode the proof bytes into the structured
        //    `GroveDBProof` AST and print its Display — the same
        //    rendering dash-evo-tool's "JSON" Proof Log mode uses
        //    (see `src/ui/tools/proof_log_screen.rs` for the
        //    reference implementation). This view shows the layered
        //    merk-proof structure inside the bytes — each layer's
        //    merk ops (Push/Parent/Child + hashes) plus the
        //    lower-layers map to descend into. The bincode config
        //    must match what grovedb's PathQuery proofs are
        //    serialized with on the wire (big-endian, no length
        //    limit) or `decode_from_slice` returns `Err`.
        let bincode_config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        match bincode::decode_from_slice::<GroveDBProof, _>(&proof, bincode_config) {
            Ok((grovedb_proof, _)) => {
                eprintln!("[proof]   proof-display:");
                for line in format!("{}", grovedb_proof).lines() {
                    eprintln!("[proof]     {line}");
                }
            }
            Err(e) => eprintln!("[proof]   proof-display decode error: {e:?}"),
        }
    }
}

/// Companion to `display_proofs` for the Count Index Group By
/// Examples chapter (G1..G6). Captures the structured proof bytes
/// the dispatcher emits for each `group_by` shape, decodes them
/// through `GroveDBProof::Display`, and tags the output with a
/// `[gproof]` prefix so the chapter's regex extraction stays
/// unambiguous.
///
/// G1 and G2 are intentionally omitted: their proof bytes are
/// byte-identical to chapter 29's Q5 / Q6 (a property the dispatcher
/// preserves because `CountMode::GroupByIn` over a single `In` clause
/// resolves to the same `point_lookup_count_path_query` as
/// `CountMode::Aggregate` does — the SDK just zips the elements with
/// the In values instead of summing). The chapter references the
/// existing Q5 / Q6 displays rather than emitting duplicate bytes.
fn display_group_by_proofs(fixture: &CountBenchFixture, platform_version: &PlatformVersion) {
    let mid_brand = brand_label(BRAND_COUNT / 2);
    let mid_color = color_label(color_count_for_rows(fixture.row_count) / 2);
    let brands_2 = brands_n(2);
    let brands_100 = brands_n(BRAND_COUNT);
    let range_floor = Value::Text(fixture.range_floor.clone());

    let clause = |field: &str, op: &str, value: Value| -> Value {
        Value::Array(vec![
            Value::Text(field.to_string()),
            Value::Text(op.to_string()),
            value,
        ])
    };

    let cases: Vec<(&str, Value, Value, CountMode, Option<u32>)> = vec![
        (
            // G1a renders alongside the rest so the chapter can quote
            // the absent-branch proof bytes and demonstrate the
            // absence subproof commitment.
            "G1a [brand] / where=brand IN[brand_000, brand_100] (one absent)",
            Value::Array(vec![clause(
                "brand",
                "in",
                Value::Array(vec![
                    Value::Text(brand_label(0)),
                    Value::Text(brand_label(BRAND_COUNT)),
                ]),
            )]),
            Value::Null,
            CountMode::GroupByIn,
            None,
        ),
        (
            "G1b [brand] / where=brand IN[100]",
            Value::Array(vec![clause(
                "brand",
                "in",
                Value::Array(brands_100.clone()),
            )]),
            Value::Null,
            CountMode::GroupByIn,
            None,
        ),
        (
            "G3 [brand] / where=brand IN[2] AND color==Y",
            Value::Array(vec![
                clause("brand", "in", Value::Array(brands_2.clone())),
                clause("color", "==", Value::Text(mid_color.clone())),
            ]),
            Value::Null,
            CountMode::GroupByIn,
            None,
        ),
        (
            "G4 [color] / where=color > floor",
            Value::Array(vec![clause("color", ">", range_floor.clone())]),
            Value::Null,
            CountMode::GroupByRange,
            None,
        ),
        (
            "G5 [brand, color] / where=brand IN[2] AND color > floor",
            Value::Array(vec![
                clause("brand", "in", Value::Array(brands_2.clone())),
                clause("color", ">", range_floor.clone()),
            ]),
            Value::Null,
            CountMode::GroupByCompound,
            None,
        ),
        (
            "G7 [brand] / where=brand IN[2] AND color > floor",
            Value::Array(vec![
                clause("brand", "in", Value::Array(brands_2.clone())),
                clause("color", ">", range_floor.clone()),
            ]),
            Value::Null,
            CountMode::GroupByIn,
            None,
        ),
        (
            "G8 [brand] / where=brand > floor AND color > floor",
            Value::Array(vec![
                clause("brand", ">", Value::Text(brand_label(BRAND_COUNT / 2))),
                clause("color", ">", range_floor.clone()),
            ]),
            Value::Null,
            CountMode::GroupByRange,
            None,
        ),
        (
            "G8a [brand] / where=brand BETWEEN AND color BETWEEN (desc)",
            Value::Array(vec![
                clause("brand", ">", Value::Text(brand_label(BRAND_COUNT / 2))),
                clause(
                    "brand",
                    "<",
                    Value::Text(brand_label(BRAND_COUNT * 65 / 100)),
                ),
                clause("color", ">", Value::Text(color_label(200))),
                clause("color", "<", Value::Text(color_label(400))),
            ]),
            Value::Array(vec![Value::Array(vec![
                Value::Text("brand".to_string()),
                Value::Text("desc".to_string()),
            ])]),
            CountMode::GroupByRange,
            None,
        ),
    ];

    let _ = mid_brand;
    let bincode_config = bincode::config::standard()
        .with_big_endian()
        .with_no_limit();

    for (label, raw_where, raw_order_by, mode, limit) in cases {
        let request = count_request(fixture, raw_where, raw_order_by, mode, limit, true);
        let proof =
            match fixture
                .drive
                .execute_document_count_request(request, None, platform_version)
            {
                Ok(DocumentCountResponse::Proof(p)) => p,
                other => {
                    eprintln!("[gproof] {label} → unexpected non-Proof response: {other:?}");
                    continue;
                }
            };

        eprintln!("[gproof] {label} ({sz} bytes)", sz = proof.len());

        match bincode::decode_from_slice::<GroveDBProof, _>(&proof, bincode_config) {
            Ok((grovedb_proof, _)) => {
                eprintln!("[gproof]   proof-display:");
                for line in format!("{grovedb_proof}").lines() {
                    eprintln!("[gproof]     {line}");
                }
            }
            Err(e) => eprintln!("[gproof]   proof-display decode error: {e:?}"),
        }
    }
}

/// Pretty-print a path or key segment: quoted UTF-8 if printable
/// ASCII, hex otherwise. Long byte strings are truncated with a
/// length suffix so the output stays scannable.
fn display_segment(bytes: &[u8]) -> String {
    if let Ok(s) = std::str::from_utf8(bytes) {
        if !s.is_empty() && s.chars().all(|c| c.is_ascii_graphic() || c == ' ') {
            return format!("{:?}", s);
        }
    }
    if bytes.is_empty() {
        return "(empty)".to_string();
    }
    if bytes.len() <= 16 {
        return format!("0x{}", hex_bytes(bytes));
    }
    let prefix = hex_bytes(&bytes[..8]);
    format!("0x{prefix}...({} bytes)", bytes.len())
}

/// Pretty-print a `Vec<QueryItem>` showing each `Key`/`Range` etc.
/// with byte segments decoded the same way as `display_segment`.
fn display_query_items(items: &[grovedb::QueryItem]) -> String {
    use grovedb::QueryItem;
    let pieces: Vec<String> = items
        .iter()
        .map(|item| match item {
            QueryItem::Key(k) => format!("Key({})", display_segment(k)),
            QueryItem::Range(r) => format!(
                "Range({}..{})",
                display_segment(&r.start),
                display_segment(&r.end)
            ),
            QueryItem::RangeInclusive(r) => format!(
                "RangeInclusive({}..={})",
                display_segment(r.start()),
                display_segment(r.end())
            ),
            QueryItem::RangeFull(_) => "RangeFull(..)".to_string(),
            QueryItem::RangeFrom(r) => format!("RangeFrom({}..)", display_segment(&r.start)),
            QueryItem::RangeTo(r) => format!("RangeTo(..{})", display_segment(&r.end)),
            QueryItem::RangeToInclusive(r) => {
                format!("RangeToInclusive(..={})", display_segment(&r.end))
            }
            QueryItem::RangeAfter(r) => format!("RangeAfter({}..)", display_segment(&r.start)),
            QueryItem::RangeAfterTo(r) => format!(
                "RangeAfterTo({}..{})",
                display_segment(&r.start),
                display_segment(&r.end)
            ),
            QueryItem::RangeAfterToInclusive(r) => format!(
                "RangeAfterToInclusive({}..={})",
                display_segment(r.start()),
                display_segment(r.end())
            ),
            QueryItem::AggregateCountOnRange(inner) => format!(
                "AggregateCountOnRange({})",
                display_query_items(std::slice::from_ref(inner))
            ),
            QueryItem::AggregateSumOnRange(inner) => format!(
                "AggregateSumOnRange({})",
                display_query_items(std::slice::from_ref(inner))
            ),
            QueryItem::AggregateCountAndSumOnRange(inner) => format!(
                "AggregateCountAndSumOnRange({})",
                display_query_items(std::slice::from_ref(inner))
            ),
        })
        .collect();
    format!("[{}]", pieces.join(", "))
}

/// Pretty-print a verified grovedb `Element`.
///
/// Distinguishes every count-bearing variant explicitly
/// (`CountTree` / `ProvableCountTree` / `CountSumTree` /
/// `ProvableCountSumTree` / `SumTree`) so a reader can tell which
/// tree shape signed the count without re-inspecting the bench
/// fixture's `primary_key_tree_type` plumbing. Also emits the
/// element's full `Debug` representation under `[proof]   debug:`
/// so the variant tag (e.g. `CountTree(None, 100000, None)` vs.
/// `ProvableCountTree(None, 100000, None)`) is unambiguous on
/// inspection — the variant choice drives whether the parent
/// `ProvableCountTree`/`CountTree` boundary signs the count and
/// matters for which verifier primitive applies upstream.
fn display_element(elem: Option<&grovedb::Element>) -> String {
    use grovedb::Element;
    match elem {
        None => "None (absent)".to_string(),
        Some(e) => {
            let count = e.count_value_or_default();
            let kind = match e {
                Element::CountTree(_, _, _) => "CountTree",
                Element::ProvableCountTree(_, _, _) => "ProvableCountTree",
                Element::SumTree(_, _, _) => "SumTree",
                Element::CountSumTree(_, _, _, _) => "CountSumTree",
                Element::ProvableCountSumTree(_, _, _, _) => "ProvableCountSumTree",
                Element::Tree(_, _) => "Tree",
                Element::Item(_, _) => "Item",
                Element::Reference(_, _, _) => "Reference",
                _ => "(other-variant)",
            };
            format!(
                "{kind} {{ count_value_or_default: {count}, debug: {:?} }}",
                e
            )
        }
    }
}

/// Compact hex helper used by `display_segment` / `display_proofs`.
fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Convenience helper for the matrix runner: run one count request
/// through the drive dispatcher and format the outcome as a short
/// string (success → describing the response shape and size; error
/// → the truncated error message). Keeps `report_group_by_matrix`'s
/// per-case body readable.
fn drive_count_outcome(
    fixture: &CountBenchFixture,
    raw_where: Value,
    raw_order_by: Value,
    mode: CountMode,
    limit: Option<u32>,
    prove: bool,
    platform_version: &PlatformVersion,
) -> String {
    let request = count_request(fixture, raw_where, raw_order_by, mode, limit, prove);
    match fixture
        .drive
        .execute_document_count_request(request, None, platform_version)
    {
        Ok(DocumentCountResponse::Aggregate(c)) => format!("Aggregate({c})"),
        Ok(DocumentCountResponse::Entries(entries)) => {
            let summed: u64 = entries.iter().filter_map(|e| e.count).sum();
            format!("Entries(len={}, sum={})", entries.len(), summed)
        }
        Ok(DocumentCountResponse::Proof(p)) => format!("Proof({} bytes)", p.len()),
        Err(e) => {
            let msg = e.to_string();
            // Truncate to keep the matrix readable; the operator
            // gist is preserved.
            let trimmed = msg
                .lines()
                .next()
                .unwrap_or(&msg)
                .chars()
                .take(120)
                .collect::<String>();
            format!("Err({trimmed})")
        }
    }
}

/// First N brands by label — convenience for matrix cases that need
/// a small In array (2-3 brands) rather than the full 100 used by
/// the criterion benches.
fn brands_n(n: u64) -> Vec<Value> {
    (0..n).map(|b| Value::Text(brand_label(b))).collect()
}

fn count_request<'a>(
    fixture: &'a CountBenchFixture,
    raw_where_value: Value,
    raw_order_by_value: Value,
    mode: CountMode,
    limit: Option<u32>,
    prove: bool,
) -> DocumentCountRequest<'a> {
    use drive::query::drive_document_count_query::drive_dispatcher::{
        order_clauses_from_value, where_clauses_from_value,
    };

    let document_type = fixture
        .data_contract
        .document_type_for_name(DOCUMENT_TYPE_NAME)
        .expect("expected widget document type");

    // The bench fixtures express where/order_by as `Value::Array`
    // shapes (matching the wire-CBOR layout). Parse them into
    // structured `Vec<WhereClause>` / `Vec<OrderClause>` here so the
    // bench keeps its compact fixture vocabulary while the
    // dispatcher consumes the same typed form the v1 ABCI handler
    // produces.
    let where_clauses = where_clauses_from_value(&raw_where_value, PlatformVersion::latest())
        .expect("bench fixture builds a valid `where` shape");
    let order_clauses = order_clauses_from_value(&raw_order_by_value)
        .expect("bench fixture builds a valid `order_by` shape");

    DocumentCountRequest {
        contract: &fixture.data_contract,
        document_type,
        where_clauses,
        order_clauses,
        mode,
        limit,
        prove,
        drive_config: &fixture.drive_config,
        resolved_time_ranges: vec![],
    }
}

fn brand_in_where_value(brands: Vec<Value>) -> Value {
    Value::Array(vec![Value::Array(vec![
        Value::Text("brand".to_string()),
        Value::Text("in".to_string()),
        Value::Array(brands),
    ])])
}

fn color_in_where_value(colors: Vec<Value>) -> Value {
    Value::Array(vec![Value::Array(vec![
        Value::Text("color".to_string()),
        Value::Text("in".to_string()),
        Value::Array(colors),
    ])])
}

/// First N colors in lex order — same naming convention as
/// `populate_fixture` (`color_NNNNNNNN`), which guarantees these
/// values exist in the fixture so the proof actually resolves
/// 100 present branches (not absent ones, which would be omitted
/// from the proof's emitted-elements stream and shrink the proof
/// trivially).
fn first_n_color_values(n: u64) -> Vec<Value> {
    (0..n)
        .map(|color| Value::Text(color_label(color)))
        .collect()
}

fn in_and_range_where_value(brands: Vec<Value>, range_floor: Value) -> Value {
    Value::Array(vec![
        Value::Array(vec![
            Value::Text("brand".to_string()),
            Value::Text("in".to_string()),
            Value::Array(brands),
        ]),
        Value::Array(vec![
            Value::Text("color".to_string()),
            Value::Text(">".to_string()),
            range_floor,
        ]),
    ])
}

fn all_brand_values() -> Vec<Value> {
    (0..BRAND_COUNT)
        .map(|brand| Value::Text(brand_label(brand)))
        .collect()
}

fn brand_label(brand: u64) -> String {
    format!("brand_{brand:03}")
}

fn color_label(color: u64) -> String {
    format!("color_{color:08}")
}

fn color_count_for_rows(row_count: u64) -> u64 {
    row_count.div_ceil(BRAND_COUNT).max(1)
}

fn document_id(row: u64) -> [u8; 32] {
    let mut id = [0u8; 32];
    let document_number = row + 1;
    id[..8].copy_from_slice(&document_number.to_be_bytes());
    id[8..16].copy_from_slice(&(!document_number).to_be_bytes());
    id
}

fn row_count() -> u64 {
    env_u64("DASH_PLATFORM_COUNT_BENCH_ROWS").unwrap_or(DEFAULT_ROW_COUNT)
}

fn batch_size() -> u64 {
    env_u64("DASH_PLATFORM_COUNT_BENCH_BATCH_SIZE").unwrap_or(DEFAULT_BATCH_SIZE)
}

fn env_u64(name: &str) -> Option<u64> {
    env::var(name)
        .ok()
        .map(|value| {
            value
                .parse::<u64>()
                .unwrap_or_else(|_| panic!("{name} must be a positive integer, got {value}"))
        })
        .filter(|value| *value > 0)
}

fn env_flag(name: &str) -> bool {
    matches!(env::var(name).as_deref(), Ok("1") | Ok("true") | Ok("TRUE"))
}

fn fixture_path(row_count: u64) -> PathBuf {
    if let Ok(path) = env::var("DASH_PLATFORM_COUNT_BENCH_DB") {
        return PathBuf::from(path);
    }

    env::temp_dir().join(format!(
        "dash-platform-document-count-bench-v{FIXTURE_SCHEMA_VERSION}-rows-{row_count}"
    ))
}

fn fixture_marker(row_count: u64) -> String {
    format!("schema_version={FIXTURE_SCHEMA_VERSION}\nrows={row_count}\nbrands={BRAND_COUNT}\n")
}

criterion_group!(count_query_worst_cases, document_count_worst_case);
criterion_main!(count_query_worst_cases);
