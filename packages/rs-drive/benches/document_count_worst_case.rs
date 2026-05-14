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
use dpp::data_contract::{DataContract, DataContractFactory};
use dpp::document::{Document, DocumentV0};
use dpp::identifier::Identifier;
use dpp::platform_value::{platform_value, Value};
use dpp::version::PlatformVersion;
use drive::config::DriveConfig;
use drive::drive::Drive;
use drive::query::{CountMode, DocumentCountRequest, DocumentCountResponse};
use drive::util::object_size_info::DocumentInfo::DocumentRefInfo;
use drive::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use drive::util::storage_flags::StorageFlags;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

const PROTOCOL_VERSION_V12: u32 = 12;
const FIXTURE_SCHEMA_VERSION: u32 = 1;
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
        mode: CountMode,
        limit: Option<u32>,
    }

    let cases: Vec<MatrixCase> = vec![
        // ── group_by = [] (Aggregate) ──────────────────────────────
        MatrixCase {
            label: "[] / where=(empty)",
            platform_allowed: "yes (documentsCountable fast path)",
            raw_where: where_empty(),
            mode: CountMode::Aggregate,
            limit: None,
        },
        MatrixCase {
            label: "[] / where=brand==X",
            platform_allowed: "yes",
            raw_where: where_brand_eq(),
            mode: CountMode::Aggregate,
            limit: None,
        },
        MatrixCase {
            label: "[] / where=color==X",
            platform_allowed: "yes",
            raw_where: where_color_eq(),
            mode: CountMode::Aggregate,
            limit: None,
        },
        MatrixCase {
            label: "[] / where=brand==X AND color==Y",
            platform_allowed: "yes",
            raw_where: where_brand_eq_color_eq(),
            mode: CountMode::Aggregate,
            limit: None,
        },
        MatrixCase {
            label: "[] / where=brand IN[2]",
            platform_allowed: "yes (per-In aggregate fan-out)",
            raw_where: where_brand_in(),
            mode: CountMode::Aggregate,
            limit: None,
        },
        MatrixCase {
            label: "[] / where=color IN[2]",
            platform_allowed: "yes (per-In aggregate fan-out)",
            raw_where: where_color_in(),
            mode: CountMode::Aggregate,
            limit: None,
        },
        MatrixCase {
            label: "[] / where=color > floor",
            platform_allowed: "yes (AggregateCountOnRange)",
            raw_where: where_color_gt(),
            mode: CountMode::Aggregate,
            limit: None,
        },
        MatrixCase {
            label: "[] / where=brand IN[2] AND color > floor",
            platform_allowed: "no-proof: yes / prove: no (aggregate proof can't fork)",
            raw_where: where_brand_in_color_gt(),
            mode: CountMode::Aggregate,
            limit: None,
        },
        // ── group_by = [color] (single-field) ──────────────────────
        MatrixCase {
            label: "[color] / where=color IN[2]",
            platform_allowed: "yes (GroupByIn)",
            raw_where: where_color_in(),
            mode: CountMode::GroupByIn,
            limit: None,
        },
        MatrixCase {
            label: "[color] / where=color > floor",
            platform_allowed: "yes (GroupByRange — distinct-range walk)",
            raw_where: where_color_gt(),
            mode: CountMode::GroupByRange,
            limit: None,
        },
        MatrixCase {
            label: "[color] / where=color==X",
            platform_allowed: "no — `color` is constrained by `==`, not `In` or range",
            raw_where: where_color_eq(),
            mode: CountMode::GroupByIn,
            limit: None,
        },
        MatrixCase {
            label: "[color] / where=brand IN[2] AND color > floor",
            platform_allowed: "no — single-field GROUP BY with both `In` and range",
            raw_where: where_brand_in_color_gt(),
            mode: CountMode::GroupByRange,
            limit: None,
        },
        // ── group_by = [brand] (single-field) ──────────────────────
        MatrixCase {
            label: "[brand] / where=brand IN[2]",
            platform_allowed: "yes (GroupByIn — non-rangeCountable byBrand)",
            raw_where: where_brand_in(),
            mode: CountMode::GroupByIn,
            limit: None,
        },
        MatrixCase {
            label: "[brand] / where=brand IN[2] AND color==Y",
            platform_allowed: "yes (GroupByIn — compound covers byBrandColor)",
            raw_where: where_brand_in_color_eq(),
            mode: CountMode::GroupByIn,
            limit: None,
        },
        MatrixCase {
            label: "[brand] / where=brand IN[2] AND color > floor",
            platform_allowed: "no — single-field GROUP BY with both `In` and range",
            raw_where: where_brand_in_color_gt(),
            mode: CountMode::GroupByIn,
            limit: None,
        },
        MatrixCase {
            label: "[brand] / where=brand==X",
            platform_allowed: "no — `brand` is `==`, not `In` or range",
            raw_where: where_brand_eq(),
            mode: CountMode::GroupByIn,
            limit: None,
        },
        // ── group_by = [brand, color] (two-field compound) ─────────
        MatrixCase {
            label: "[brand, color] / where=brand IN[2] AND color > floor",
            platform_allowed: "yes (GroupByCompound — `(In, range)` shape)",
            raw_where: where_brand_in_color_gt(),
            mode: CountMode::GroupByCompound,
            limit: Some(100),
        },
        MatrixCase {
            label: "[brand, color] / where=brand IN[2] AND color==Y",
            platform_allowed: "no — `color` must be range, not `==`",
            raw_where: where_brand_in_color_eq(),
            mode: CountMode::GroupByCompound,
            limit: Some(100),
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
            mode: CountMode::GroupByCompound,
            limit: Some(100),
        },
    ];

    for case in &cases {
        let noproof_result = drive_count_outcome(
            fixture,
            case.raw_where.clone(),
            case.mode,
            case.limit,
            false,
            platform_version,
        );
        let prove_result = drive_count_outcome(
            fixture,
            case.raw_where.clone(),
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

/// Convenience helper for the matrix runner: run one count request
/// through the drive dispatcher and format the outcome as a short
/// string (success → describing the response shape and size; error
/// → the truncated error message). Keeps `report_group_by_matrix`'s
/// per-case body readable.
fn drive_count_outcome(
    fixture: &CountBenchFixture,
    raw_where: Value,
    mode: CountMode,
    limit: Option<u32>,
    prove: bool,
    platform_version: &PlatformVersion,
) -> String {
    let request = count_request(fixture, raw_where, Value::Null, mode, limit, prove);
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
    let document_type = fixture
        .data_contract
        .document_type_for_name(DOCUMENT_TYPE_NAME)
        .expect("expected widget document type");

    DocumentCountRequest {
        contract: &fixture.data_contract,
        document_type,
        raw_where_value,
        raw_order_by_value,
        mode,
        limit,
        prove,
        drive_config: &fixture.drive_config,
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
