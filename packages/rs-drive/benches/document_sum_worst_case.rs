//! Worst-case benchmarks for the document-sum query paths proposed by
//! `GetDocumentsSumRequestV1` (the sum analog of `GetDocumentsRequestV1`'s
//! count surface — see `document_count_worst_case.rs`).
//!
//! The fixture intentionally uses Drive's normal contract application and
//! document insertion path so the resulting GroveDB contains the same primary
//! trees, summable index trees, and range-summable index trees as production
//! once the sum-tree feature lands.
//!
//! Status: this bench depends on schema-level sum-index syntax (`documentsSummable`,
//! `summable`, `rangeSummable`) and the `DriveDocumentSumQuery` family that are
//! described in [`book/src/drive/document-sum-trees.md`](../../../../book/src/drive/document-sum-trees.md)
//! but not yet wired through DPP and Drive. The bench is committed as an
//! executable design spec — once the feature lands, this file compiles and
//! produces the numbers that backfill the TBDs in
//! [`book/src/drive/sum-index-examples.md`](../../../../book/src/drive/sum-index-examples.md).
//!
//! Environment knobs:
//! - `DASH_PLATFORM_SUM_BENCH_ROWS`: row count to build; defaults to 100,000.
//! - `DASH_PLATFORM_SUM_BENCH_DB`: fixture directory; defaults under `std::env::temp_dir()`.
//! - `DASH_PLATFORM_SUM_BENCH_REBUILD=1`: remove and rebuild the fixture.
//! - `DASH_PLATFORM_SUM_BENCH_BATCH_SIZE`: inserts per transaction; defaults to 10,000.

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
// NOTE: these types are the proposed sum-query surface. They don't exist
// in `drive::query` yet — landing them is part of the sum-tree feature.
// Named to parallel the count surface (`DriveDocumentCountQuery`,
// `DocumentCountRequest`, `DocumentCountResponse`, `CountMode`).
use drive::query::{
    DocumentSumRequest, DocumentSumResponse, DriveDocumentSumQuery, SumMode, WhereClause,
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
// invalidates a cached `tmp/dash-platform-document-sum-bench-v{N}-rows-…`
// directory.
const FIXTURE_SCHEMA_VERSION: u32 = 1;
const DEFAULT_ROW_COUNT: u64 = 100_000;
const DEFAULT_BATCH_SIZE: u64 = 10_000;
const RECIPIENT_COUNT: u64 = 100;
const DOCUMENT_TYPE_NAME: &str = "tip";
const SUM_PROPERTY_NAME: &str = "amount";
const READY_MARKER: &str = ".document-sum-worst-case-ready";

struct SumBenchFixture {
    drive: Drive,
    data_contract: DataContract,
    drive_config: DriveConfig,
    row_count: u64,
    /// Bench's standard range floor — the midpoint of the sentAt
    /// timeline. `sentAt > range_floor` (Query 7) crosses exactly half
    /// the rows, producing a predictable sum target.
    range_floor: u64,
}

impl SumBenchFixture {
    fn load_or_create() -> Self {
        let row_count = row_count();
        let fixture_path = fixture_path(row_count);
        let rebuild = env_flag("DASH_PLATFORM_SUM_BENCH_REBUILD");
        let ready_marker = fixture_path.join(READY_MARKER);
        let expected_marker = fixture_marker(row_count);

        if rebuild && fixture_path.exists() {
            fs::remove_dir_all(&fixture_path).expect("expected to remove old sum bench fixture");
        }

        let data_contract = tip_jar_contract();
        let drive_config = DriveConfig::default();

        if ready_marker.exists()
            && fs::read_to_string(&ready_marker).expect("expected to read sum bench fixture marker")
                == expected_marker
        {
            eprintln!(
                "reusing document-sum fixture at {} with {} rows",
                fixture_path.display(),
                row_count
            );
            let (drive, _) = Drive::open(&fixture_path, Some(drive_config.clone()))
                .expect("expected to open existing sum bench fixture");
            return Self::new(drive, data_contract, drive_config, row_count);
        }

        if fixture_path.exists() {
            fs::remove_dir_all(&fixture_path)
                .expect("expected to remove incomplete sum bench fixture");
        }
        fs::create_dir_all(&fixture_path).expect("expected to create sum bench fixture dir");

        eprintln!(
            "building document-sum fixture at {} with {} rows",
            fixture_path.display(),
            row_count
        );

        let started = Instant::now();
        let platform_version = PlatformVersion::latest();
        let (drive, _) = Drive::open(&fixture_path, Some(drive_config.clone()))
            .expect("expected to open new sum bench fixture");

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
            .expect("expected to apply sum bench contract");

        populate_fixture(&drive, &data_contract, row_count, platform_version);
        fs::write(&ready_marker, expected_marker)
            .expect("expected to mark sum bench fixture ready");

        eprintln!(
            "built document-sum fixture with {} rows in {:.2?}",
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
        // sentAt = row, so the midpoint is row_count/2.
        let range_floor = row_count / 2;

        Self {
            drive,
            data_contract,
            drive_config,
            row_count,
            range_floor,
        }
    }
}

/// The tip-jar contract — canonical schema lives in
/// `packages/rs-drive/tests/supporting_files/contract/tip-jar/tip-jar-contract.json`.
/// Mirrored inline here as a `platform_value!` literal so the bench owns
/// its own contract construction (matching `widget_contract()`'s pattern
/// in the count bench).
///
/// Three indexes parallel the widget contract's three:
/// - `byRecipient` (summable only)         ↔ widget's `byBrand`
/// - `bySentAt` (summable + rangeSummable) ↔ widget's `byColor`
/// - `byRecipientTime` (summable + rangeSummable, compound) ↔ widget's `byBrandColor`
fn tip_jar_contract() -> DataContract {
    let factory =
        DataContractFactory::new(PROTOCOL_VERSION_V12).expect("expected to create factory");
    let document_schema = platform_value!({
        "type": "object",
        "documentsMutable": false,
        "documentsSummable": "amount",
        "properties": {
            "recipient": {
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "position": 0,
                "contentMediaType": "application/x.dash.dpp.identifier"
            },
            // `maximum` bounds the property to u32::MAX so DPP infers
            // `U32` (an accepted summable type) — U64 is rejected
            // because it would overflow grovedb's i64 sum aggregator.
            "amount": {"type": "integer", "minimum": 1, "maximum": 4294967295i64, "position": 1},
            "sentAt": {"type": "integer", "minimum": 0, "position": 2},
            "note": {"type": "string", "maxLength": 280, "position": 3}
        },
        "required": ["recipient", "amount", "sentAt"],
        "indices": [
            {
                "name": "byRecipient",
                "properties": [{"recipient": "asc"}],
                "summable": "amount"
            },
            {
                "name": "bySentAt",
                "properties": [{"sentAt": "asc"}],
                "summable": "amount",
                "rangeSummable": true
            },
            {
                "name": "byRecipientTime",
                "properties": [{"recipient": "asc"}, {"sentAt": "asc"}],
                "summable": "amount",
                "rangeSummable": true
            }
        ],
        "additionalProperties": false
    });
    let schemas = platform_value!({ DOCUMENT_TYPE_NAME: document_schema });

    factory
        .create_with_value_config(Identifier::from([42u8; 32]), 0, schemas, None, None)
        .expect("expected to create sum bench data contract")
        .data_contract_owned()
}

/// Deterministic insert schedule, mirroring widget's `(brand_(row %
/// 100), color_(row / 100), serial=row)` pattern:
///
///   row → (recipient = recipient_id(row % RECIPIENT_COUNT),
///          sentAt    = row,
///          amount    = (row % 10) + 1)
///
/// This gives:
/// - exactly `row_count / RECIPIENT_COUNT` tips per recipient
///   (1 000 per recipient at the default 100k rows),
/// - a periodic `amount` distribution of `[1..10]` repeating
///   `row_count / 10` times across the global timeline,
/// - per-recipient `sum(amount)` = `(row_count / RECIPIENT_COUNT / 10)
///   × (1+2+…+10) = (row_count / RECIPIENT_COUNT / 10) × 55`,
///   = **5 500** at 100k rows.
/// - total `sum(amount)` = `(row_count / 10) × 55 = row_count × 5.5`,
///   = **550 000** at 100k rows.
fn populate_fixture(
    drive: &Drive,
    data_contract: &DataContract,
    row_count: u64,
    platform_version: &PlatformVersion,
) {
    let document_type = data_contract
        .document_type_for_name(DOCUMENT_TYPE_NAME)
        .expect("expected tip document type");
    let batch_size = batch_size();
    let recipients: Vec<[u8; 32]> = (0..RECIPIENT_COUNT).map(recipient_id).collect();

    let mut next_row = 0;
    while next_row < row_count {
        let end_row = (next_row + batch_size).min(row_count);
        let transaction = drive.grove.start_transaction();

        for row in next_row..end_row {
            let recipient = recipients[(row % RECIPIENT_COUNT) as usize];
            let sent_at = row;
            let amount: u64 = (row % 10) + 1;
            insert_tip_document(
                drive,
                data_contract,
                document_type,
                row,
                recipient,
                sent_at,
                amount,
                Some(&transaction),
                platform_version,
            );
        }

        drive
            .grove
            .commit_transaction(transaction)
            .value
            .expect("expected sum bench insert transaction to commit");

        next_row = end_row;
        if next_row == row_count || next_row % 100_000 == 0 {
            eprintln!("inserted {next_row}/{row_count} sum bench rows");
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn insert_tip_document(
    drive: &Drive,
    data_contract: &DataContract,
    document_type: dpp::data_contract::document_type::DocumentTypeRef,
    row: u64,
    recipient: [u8; 32],
    sent_at: u64,
    amount: u64,
    transaction: grovedb::TransactionArg,
    platform_version: &PlatformVersion,
) {
    let mut properties = BTreeMap::new();
    properties.insert("recipient".to_string(), Value::Bytes(recipient.to_vec()));
    properties.insert("amount".to_string(), Value::U64(amount));
    properties.insert("sentAt".to_string(), Value::U64(sent_at));

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
        .expect("expected to insert sum bench document");
}

fn document_sum_worst_case(c: &mut Criterion) {
    let fixture = SumBenchFixture::load_or_create();
    let platform_version = PlatformVersion::latest();
    let recipients = all_recipient_values();
    let broad_range_floor = Value::U64(fixture.range_floor);

    // One-shot proof-size report. Criterion measures time, but for
    // sum-proof work the load-bearing number is bytes-per-proof —
    // an optimization that shaves a merk layer (e.g. the
    // rangeSummable terminator's `[0]` descent) drops proof size
    // linearly with the number of resolved branches while leaving
    // wall-clock per-proof time roughly unchanged on warm caches.
    // Print sizes once at bench setup so reviewers can compare
    // before/after numbers from the same fixture without parsing
    // criterion's HTML output.
    report_proof_sizes(&fixture, &recipients, &broad_range_floor, platform_version);

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
    // verified payload (root hash + sum/elements). The path
    // query is the prover-side spec and the verified payload is
    // what `GroveDb::verify_query` / `verify_aggregate_sum_query`
    // reconstructs after walking the proof — together they make
    // the proof's *meaning* legible without staring at hex.
    display_proofs(&fixture, platform_version);

    // Empirical probe of the value-tree element type for the two
    // single-property index terminators in the bench's contract
    // (`byRecipient` is just `summable`, `bySentAt` is `rangeSummable`).
    // Surfaces the structural asymmetry that gates the
    // rangeSummable optimization — same shape as count's
    // probe_value_tree_types.
    probe_value_tree_types(&fixture, platform_version);

    let mut group = c.benchmark_group("document_sum_worst_case");
    group.sample_size(10);
    group.throughput(criterion::Throughput::Elements(fixture.row_count));

    group.bench_function("group_by_in_proof_100_sum_tree_branches", |b| {
        let raw_where = recipient_in_where_value(recipients.clone());
        b.iter_batched(
            || {
                sum_request(
                    &fixture,
                    SUM_PROPERTY_NAME,
                    raw_where.clone(),
                    Value::Null,
                    SumMode::GroupByIn,
                    None,
                    true,
                )
            },
            |request| match fixture
                .drive
                .execute_document_sum_request(request, None, platform_version)
                .expect("expected group_by In proof sum request")
            {
                DocumentSumResponse::Proof(proof) => black_box(proof),
                response => panic!("expected proof response, got {response:?}"),
            },
            BatchSize::SmallInput,
        );
    });

    // Rangesummable-terminator variant of the In-grouped proof. The
    // contract's `bySentAt` index is `rangeSummable: true`, so the
    // covering value trees are themselves SumTrees and the
    // point-lookup builder skips the `[0]` descent (see
    // `point_lookup_sum_path_query`'s "two terminator shapes"
    // section). Pairs with `group_by_in_proof_100_sum_tree_branches`
    // (which targets the non-range_summable `byRecipient` index) to
    // surface the optimization's per-branch byte savings.
    let sent_ats = first_n_sent_at_values(RECIPIENT_COUNT);
    group.bench_function(
        "group_by_sent_at_in_proof_100_rangesummable_branches",
        |b| {
            let raw_where = sent_at_in_where_value(sent_ats.clone());
            b.iter_batched(
                || {
                    sum_request(
                        &fixture,
                        SUM_PROPERTY_NAME,
                        raw_where.clone(),
                        Value::Null,
                        SumMode::GroupByIn,
                        None,
                        true,
                    )
                },
                |request| match fixture
                    .drive
                    .execute_document_sum_request(request, None, platform_version)
                    .expect("expected group_by sentAt-In proof sum request")
                {
                    DocumentSumResponse::Proof(proof) => black_box(proof),
                    response => panic!("expected proof response, got {response:?}"),
                },
                BatchSize::SmallInput,
            );
        },
    );

    group.bench_function("aggregate_in_range_no_proof_100_range_sums", |b| {
        let raw_where = in_and_range_where_value(recipients.clone(), broad_range_floor.clone());
        b.iter_batched(
            || {
                sum_request(
                    &fixture,
                    SUM_PROPERTY_NAME,
                    raw_where.clone(),
                    Value::Null,
                    SumMode::Aggregate,
                    None,
                    false,
                )
            },
            |request| match fixture
                .drive
                .execute_document_sum_request(request, None, platform_version)
                .expect("expected aggregate In+range sum request")
            {
                DocumentSumResponse::Aggregate(sum) => black_box(sum),
                response => panic!("expected aggregate response, got {response:?}"),
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("group_by_compound_in_range_no_proof_limit_100", |b| {
        let raw_where = in_and_range_where_value(recipients.clone(), broad_range_floor.clone());
        b.iter_batched(
            || {
                sum_request(
                    &fixture,
                    SUM_PROPERTY_NAME,
                    raw_where.clone(),
                    Value::Null,
                    SumMode::GroupByCompound,
                    Some(100),
                    false,
                )
            },
            |request| match fixture
                .drive
                .execute_document_sum_request(request, None, platform_version)
                .expect("expected compound no-proof sum request")
            {
                DocumentSumResponse::Entries(entries) => black_box(entries),
                response => panic!("expected entries response, got {response:?}"),
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("group_by_compound_in_range_proof_limit_100", |b| {
        let raw_where = in_and_range_where_value(recipients.clone(), broad_range_floor.clone());
        b.iter_batched(
            || {
                sum_request(
                    &fixture,
                    SUM_PROPERTY_NAME,
                    raw_where.clone(),
                    Value::Null,
                    SumMode::GroupByCompound,
                    Some(100),
                    true,
                )
            },
            |request| match fixture
                .drive
                .execute_document_sum_request(request, None, platform_version)
                .expect("expected compound proof sum request")
            {
                DocumentSumResponse::Proof(proof) => black_box(proof),
                response => panic!("expected proof response, got {response:?}"),
            },
            BatchSize::SmallInput,
        );
    });

    // Per-query timing for the 8 chapter queries (no group_by). Each
    // case exercises the same proof shape documented in
    // `book/src/drive/sum-index-examples.md` so reviewers can quote
    // wall-clock timings alongside the proof-size and complexity
    // columns in the chapter's overview table.
    let mid_recipient = recipient_id(RECIPIENT_COUNT / 2);
    let mid_sent_at = fixture.row_count / 2;
    let recipients_2 = recipients_n(2);
    let sent_ats_2 = first_n_sent_at_values(2);
    let clause = |field: &str, op: &str, value: Value| -> Value {
        Value::Array(vec![
            Value::Text(field.to_string()),
            Value::Text(op.to_string()),
            value,
        ])
    };

    let chapter_queries: Vec<(&str, Value)> = vec![
        ("query_1_empty_total_sum", Value::Null),
        (
            "query_2_recipient_eq",
            Value::Array(vec![clause(
                "recipient",
                "==",
                Value::Bytes(mid_recipient.to_vec()),
            )]),
        ),
        (
            "query_3_sent_at_eq",
            Value::Array(vec![clause("sentAt", "==", Value::U64(mid_sent_at))]),
        ),
        (
            "query_4_recipient_eq_and_sent_at_eq",
            Value::Array(vec![
                clause("recipient", "==", Value::Bytes(mid_recipient.to_vec())),
                clause("sentAt", "==", Value::U64(mid_sent_at)),
            ]),
        ),
        (
            "query_5_recipient_in_2",
            Value::Array(vec![clause(
                "recipient",
                "in",
                Value::Array(recipients_2.clone()),
            )]),
        ),
        (
            "query_6_sent_at_in_2",
            Value::Array(vec![clause(
                "sentAt",
                "in",
                Value::Array(sent_ats_2.clone()),
            )]),
        ),
        (
            "query_7_sent_at_gt_floor",
            Value::Array(vec![clause("sentAt", ">", broad_range_floor.clone())]),
        ),
        (
            "query_8_recipient_eq_and_sent_at_gt_floor",
            Value::Array(vec![
                clause("recipient", "==", Value::Bytes(mid_recipient.to_vec())),
                clause("sentAt", ">", broad_range_floor.clone()),
            ]),
        ),
    ];

    for (name, raw_where) in chapter_queries {
        group.bench_function(name, |b| {
            b.iter_batched(
                || {
                    sum_request(
                        &fixture,
                        SUM_PROPERTY_NAME,
                        raw_where.clone(),
                        Value::Null,
                        SumMode::Aggregate,
                        None,
                        true,
                    )
                },
                |request| match fixture
                    .drive
                    .execute_document_sum_request(request, None, platform_version)
                    .expect("expected proof response for chapter query")
                {
                    DocumentSumResponse::Proof(proof) => black_box(proof),
                    response => panic!("expected proof response, got {response:?}"),
                },
                BatchSize::SmallInput,
            );
        });
    }

    // Per-query timing for the Sum Index Group By Examples chapter
    // (G1 through G5 — the basic shapes that mirror count's group-by
    // chapter). More exotic carrier shapes (G7/G8/etc. from count)
    // are not included here; sum carriers are a follow-up once the
    // basic group_by surface lands.
    let recipients_100 = recipients_n(RECIPIENT_COUNT);
    let order_by_recipient_desc = Value::Array(vec![Value::Array(vec![
        Value::Text("recipient".to_string()),
        Value::Text("desc".to_string()),
    ])]);
    let groupby_chapter_queries: Vec<(&str, Value, Value, SumMode, Option<u32>)> = vec![
        (
            "query_g1_recipient_in_grouped_by_recipient",
            Value::Array(vec![clause(
                "recipient",
                "in",
                Value::Array(recipients_2.clone()),
            )]),
            Value::Null,
            SumMode::GroupByIn,
            None,
        ),
        (
            // G1a: same `In on byRecipient` shape as G1 but one of the
            // In values is absent from the fixture (RECIPIENT_COUNT =
            // 100, so recipient_ids are 0..99). Captures the
            // absent-branch proof shape — the grovedb proof still
            // commits an absence subproof at the missing key, but
            // `verify_query` without
            // `absence_proofs_for_non_existing_searched_keys: true`
            // drops the absent branch from the returned entries.
            "query_g1a_recipient_in_with_absent_grouped_by_recipient",
            Value::Array(vec![clause(
                "recipient",
                "in",
                Value::Array(vec![
                    Value::Bytes(recipient_id(0).to_vec()),
                    Value::Bytes(recipient_id(RECIPIENT_COUNT).to_vec()),
                ]),
            )]),
            Value::Null,
            SumMode::GroupByIn,
            None,
        ),
        (
            // G1b: same shape as G1, scaled to |IN| = RECIPIENT_COUNT
            // = 100. The proof reveals every byRecipient entry as a
            // `KVValueHashFeatureTypeWithChildHash` target — the
            // most efficient byte-per-key shape `GroupByIn` can hit.
            "query_g1b_recipient_in_100_grouped_by_recipient",
            Value::Array(vec![clause(
                "recipient",
                "in",
                Value::Array(recipients_100.clone()),
            )]),
            Value::Null,
            SumMode::GroupByIn,
            None,
        ),
        (
            "query_g2_sent_at_in_grouped_by_sent_at",
            Value::Array(vec![clause(
                "sentAt",
                "in",
                Value::Array(sent_ats_2.clone()),
            )]),
            Value::Null,
            SumMode::GroupByIn,
            None,
        ),
        (
            "query_g3_recipient_in_sent_at_eq_grouped_by_recipient",
            Value::Array(vec![
                clause("recipient", "in", Value::Array(recipients_2.clone())),
                clause("sentAt", "==", Value::U64(mid_sent_at)),
            ]),
            Value::Null,
            SumMode::GroupByIn,
            None,
        ),
        (
            "query_g4_sent_at_gt_grouped_by_sent_at",
            Value::Array(vec![clause("sentAt", ">", broad_range_floor.clone())]),
            Value::Null,
            SumMode::GroupByRange,
            None,
        ),
        (
            "query_g5_recipient_in_sent_at_gt_grouped_by_recipient_sent_at",
            Value::Array(vec![
                clause("recipient", "in", Value::Array(recipients_2.clone())),
                clause("sentAt", ">", broad_range_floor.clone()),
            ]),
            Value::Null,
            SumMode::GroupByCompound,
            None,
        ),
        (
            // Descending-order variant: matches what
            // `order_clauses_from_value` parses into a single
            // `OrderClause { field: recipient, ascending: false }`.
            // The dispatcher reads the first order clause's direction
            // to pick `left_to_right` for the group-by walk.
            "query_g4_desc_sent_at_gt_grouped_by_sent_at_desc",
            Value::Array(vec![clause("sentAt", ">", broad_range_floor.clone())]),
            order_by_recipient_desc.clone(),
            SumMode::GroupByRange,
            None,
        ),
    ];

    for (name, raw_where, raw_order_by, mode, limit) in groupby_chapter_queries {
        group.bench_function(name, |b| {
            b.iter_batched(
                || {
                    sum_request(
                        &fixture,
                        SUM_PROPERTY_NAME,
                        raw_where.clone(),
                        raw_order_by.clone(),
                        mode,
                        limit,
                        true,
                    )
                },
                |request| match fixture
                    .drive
                    .execute_document_sum_request(request, None, platform_version)
                    .expect("expected proof response for group_by chapter query")
                {
                    DocumentSumResponse::Proof(proof) => black_box(proof),
                    response => panic!("expected proof response, got {response:?}"),
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Median-of-N wall-clock timing for a closure that produces a
/// proof (or any byte payload). One warmup iteration discards the
/// cold-cache hit; the remaining `iters` samples feed a sort+pick
/// median. Returns the median `Duration`. Used by `report_proof_sizes`
/// and `display_proofs` to publish per-query "Avg time" numbers into
/// the [Sum Index Examples](../../../../../book/src/drive/sum-index-examples.md)
/// chapter without spinning up a full Criterion harness per case
/// (Criterion's already running on the load-bearing N=100 shapes;
/// this is for Q1–Q9 where each shape is exercised exactly once).
fn time_median<F: FnMut()>(iters: usize, mut f: F) -> std::time::Duration {
    // Warmup — first call usually pays a cold rocksdb cache miss
    // and would skew the median heavily.
    f();
    let mut samples: Vec<std::time::Duration> = Vec::with_capacity(iters);
    for _ in 0..iters {
        let t = Instant::now();
        f();
        samples.push(t.elapsed());
    }
    samples.sort();
    samples[samples.len() / 2]
}

/// Run each proof-emitting shape once and print the resulting
/// `Vec<u8>` length plus a median wall-clock time. Criterion still
/// drives the N=100 throughput shapes; this is the lightweight
/// per-case probe that publishes "Avg time" numbers for the
/// chapter's Q1–Q9 table.
fn report_proof_sizes(
    fixture: &SumBenchFixture,
    recipients: &[Value],
    broad_range_floor: &Value,
    platform_version: &PlatformVersion,
) {
    let sent_ats_100 = first_n_sent_at_values(RECIPIENT_COUNT);
    let cases: [(&str, Value, Value, SumMode, Option<u32>); 3] = [
        // Non-rangeSummable `byRecipient` In-grouped proof — control.
        (
            "group_by_in_proof_100_sum_tree_branches",
            recipient_in_where_value(recipients.to_vec()),
            Value::Null,
            SumMode::GroupByIn,
            None,
        ),
        // RangeSummable `bySentAt` In-grouped proof — the shape the
        // optimization targets. Outer Keys resolve directly to the
        // value-tree SumTrees (no `[0]` descent), so this proof is
        // strictly smaller than the non-range_summable variant
        // above on the same fixture.
        (
            "group_by_sent_at_in_proof_100_rangesummable_branches",
            sent_at_in_where_value(sent_ats_100),
            Value::Null,
            SumMode::GroupByIn,
            None,
        ),
        (
            "group_by_compound_in_range_proof_limit_100",
            in_and_range_where_value(recipients.to_vec(), broad_range_floor.clone()),
            Value::Null,
            SumMode::GroupByCompound,
            Some(100),
        ),
    ];

    for (name, raw_where, raw_order_by, mode, limit) in cases {
        // Soft-skip cases that surface NotSupported / Unsupported so
        // partial coverage doesn't block the rest of the report.
        // Carrier-sum proofs work as of grovedb PR #670 head
        // `e98bab5f`; the remaining typical skip cause is the
        // group-by-range / group-by-compound distinct walker, which
        // is the next sum-side port (mirror of count's
        // `distinct_count_path_query`).
        let make_request = || {
            sum_request(
                fixture,
                SUM_PROPERTY_NAME,
                raw_where.clone(),
                raw_order_by.clone(),
                mode,
                limit,
                true,
            )
        };
        match fixture
            .drive
            .execute_document_sum_request(make_request(), None, platform_version)
        {
            Ok(DocumentSumResponse::Proof(proof)) => {
                // Median-of-5 wall-clock: warmup discarded inside
                // `time_median`. The closure rebuilds the request
                // each iteration so we measure the executor +
                // grovedb prover end-to-end on the same shape the
                // dispatcher sees from the wire.
                let median = time_median(5, || {
                    let _ = fixture.drive.execute_document_sum_request(
                        make_request(),
                        None,
                        platform_version,
                    );
                });
                eprintln!(
                    "[proof-size] rows={} {}: {} bytes  median={:.1} µs",
                    fixture.row_count,
                    name,
                    proof.len(),
                    median.as_secs_f64() * 1_000_000.0,
                );
            }
            Ok(other) => panic!("expected Proof response for {name}, got {other:?}"),
            Err(e) => {
                let msg = format!("{e:?}");
                let truncated: String = msg.chars().take(160).collect();
                eprintln!(
                    "[proof-size] rows={} {}: skipped — {}",
                    fixture.row_count, name, truncated
                );
            }
        }
    }
}

/// Run every `(group_by × where_shape)` combination of interest
/// through the drive sum dispatcher and report whether each works
/// on the no-proof and prove paths.
///
/// **Drive vs. platform layer.** This is the drive-level dispatcher
/// (`Drive::execute_document_sum_request`); the platform-level
/// handler (`drive-abci::query_documents_sum_v1` →
/// `validate_and_route`) layers additional validation on top.
/// Where the platform layer rejects a combination the drive layer
/// would technically accept, that's flagged in the `[matrix]`
/// output's annotations.
///
/// Output is `[matrix] {key} = {result}` lines so callers can grep
/// them out of the bench's stderr stream.
fn report_group_by_matrix(fixture: &SumBenchFixture, platform_version: &PlatformVersion) {
    let recipients_2: Vec<Value> = recipients_n(2);
    let sent_ats_2: Vec<Value> = first_n_sent_at_values(2);
    let mid_recipient = recipient_id(RECIPIENT_COUNT / 2);
    let mid_sent_at = fixture.row_count / 2;
    let range_floor = Value::U64(fixture.range_floor);

    // Compact builder for where-clause `Value::Array`s. Each inner
    // array is `[field, op, value]` — the wire shape the drive
    // dispatcher parses via `parse_sum_where_value`.
    let clause = |field: &str, op: &str, value: Value| -> Value {
        Value::Array(vec![
            Value::Text(field.to_string()),
            Value::Text(op.to_string()),
            value,
        ])
    };
    let where_empty = || Value::Null;
    let where_recipient_in = || {
        Value::Array(vec![clause(
            "recipient",
            "in",
            Value::Array(recipients_2.clone()),
        )])
    };
    let where_sent_at_in = || {
        Value::Array(vec![clause(
            "sentAt",
            "in",
            Value::Array(sent_ats_2.clone()),
        )])
    };
    let where_recipient_eq = || {
        Value::Array(vec![clause(
            "recipient",
            "==",
            Value::Bytes(mid_recipient.to_vec()),
        )])
    };
    let where_sent_at_eq = || Value::Array(vec![clause("sentAt", "==", Value::U64(mid_sent_at))]);
    let where_recipient_eq_sent_at_eq = || {
        Value::Array(vec![
            clause("recipient", "==", Value::Bytes(mid_recipient.to_vec())),
            clause("sentAt", "==", Value::U64(mid_sent_at)),
        ])
    };
    let where_sent_at_gt = || Value::Array(vec![clause("sentAt", ">", range_floor.clone())]);
    let where_recipient_in_sent_at_gt = || {
        Value::Array(vec![
            clause("recipient", "in", Value::Array(recipients_2.clone())),
            clause("sentAt", ">", range_floor.clone()),
        ])
    };
    let where_recipient_in_sent_at_eq = || {
        Value::Array(vec![
            clause("recipient", "in", Value::Array(recipients_2.clone())),
            clause("sentAt", "==", Value::U64(mid_sent_at)),
        ])
    };
    let where_recipient_eq_sent_at_gt = || {
        Value::Array(vec![
            clause("recipient", "==", Value::Bytes(mid_recipient.to_vec())),
            clause("sentAt", ">", range_floor.clone()),
        ])
    };

    struct MatrixCase {
        label: &'static str,
        platform_allowed: &'static str,
        raw_where: Value,
        raw_order_by: Value,
        mode: SumMode,
        limit: Option<u32>,
    }

    let cases: Vec<MatrixCase> = vec![
        // ── group_by = [] (Aggregate) ──────────────────────────────
        MatrixCase {
            label: "[] / where=(empty)",
            platform_allowed: "yes (documentsSummable fast path)",
            raw_where: where_empty(),
            raw_order_by: Value::Null,
            mode: SumMode::Aggregate,
            limit: None,
        },
        MatrixCase {
            label: "[] / where=recipient==X",
            platform_allowed: "yes",
            raw_where: where_recipient_eq(),
            raw_order_by: Value::Null,
            mode: SumMode::Aggregate,
            limit: None,
        },
        MatrixCase {
            label: "[] / where=sentAt==X",
            platform_allowed: "yes",
            raw_where: where_sent_at_eq(),
            raw_order_by: Value::Null,
            mode: SumMode::Aggregate,
            limit: None,
        },
        MatrixCase {
            label: "[] / where=recipient==X AND sentAt==Y",
            platform_allowed: "yes",
            raw_where: where_recipient_eq_sent_at_eq(),
            raw_order_by: Value::Null,
            mode: SumMode::Aggregate,
            limit: None,
        },
        MatrixCase {
            label: "[] / where=recipient IN[2]",
            platform_allowed: "yes (per-In aggregate fan-out)",
            raw_where: where_recipient_in(),
            raw_order_by: Value::Null,
            mode: SumMode::Aggregate,
            limit: None,
        },
        MatrixCase {
            label: "[] / where=sentAt IN[2]",
            platform_allowed: "yes (per-In aggregate fan-out)",
            raw_where: where_sent_at_in(),
            raw_order_by: Value::Null,
            mode: SumMode::Aggregate,
            limit: None,
        },
        MatrixCase {
            label: "[] / where=sentAt > floor",
            platform_allowed: "yes (AggregateSumOnRange)",
            raw_where: where_sent_at_gt(),
            raw_order_by: Value::Null,
            mode: SumMode::Aggregate,
            limit: None,
        },
        MatrixCase {
            label: "[] / where=recipient==X AND sentAt > floor",
            platform_allowed: "yes (AggregateSumOnRange on byRecipientTime terminator)",
            raw_where: where_recipient_eq_sent_at_gt(),
            raw_order_by: Value::Null,
            mode: SumMode::Aggregate,
            limit: None,
        },
        MatrixCase {
            label: "[] / where=recipient IN[2] AND sentAt > floor",
            platform_allowed: "no-proof: yes / prove: no (aggregate proof can't fork)",
            raw_where: where_recipient_in_sent_at_gt(),
            raw_order_by: Value::Null,
            mode: SumMode::Aggregate,
            limit: None,
        },
        // ── group_by = [sentAt] (single-field) ─────────────────────
        MatrixCase {
            label: "[sentAt] / where=sentAt IN[2]",
            platform_allowed: "yes (GroupByIn)",
            raw_where: where_sent_at_in(),
            raw_order_by: Value::Null,
            mode: SumMode::GroupByIn,
            limit: None,
        },
        MatrixCase {
            label: "[sentAt] / where=sentAt > floor",
            platform_allowed: "yes (GroupByRange — distinct-range walk)",
            raw_where: where_sent_at_gt(),
            raw_order_by: Value::Null,
            mode: SumMode::GroupByRange,
            limit: None,
        },
        MatrixCase {
            label: "[sentAt] / where=sentAt==X",
            platform_allowed: "no — `sentAt` is constrained by `==`, not `In` or range",
            raw_where: where_sent_at_eq(),
            raw_order_by: Value::Null,
            mode: SumMode::GroupByIn,
            limit: None,
        },
        // ── group_by = [recipient] (single-field) ──────────────────
        MatrixCase {
            label: "[recipient] / where=recipient IN[2]",
            platform_allowed: "yes (GroupByIn — non-rangeSummable byRecipient)",
            raw_where: where_recipient_in(),
            raw_order_by: Value::Null,
            mode: SumMode::GroupByIn,
            limit: None,
        },
        MatrixCase {
            label: "[recipient] / where=recipient IN[2] AND sentAt==Y",
            platform_allowed: "yes (GroupByIn — compound covers byRecipientTime)",
            raw_where: where_recipient_in_sent_at_eq(),
            raw_order_by: Value::Null,
            mode: SumMode::GroupByIn,
            limit: None,
        },
        MatrixCase {
            label: "[recipient] / where=recipient==X",
            platform_allowed: "no — `recipient` is `==`, not `In` or range",
            raw_where: where_recipient_eq(),
            raw_order_by: Value::Null,
            mode: SumMode::GroupByIn,
            limit: None,
        },
        // ── group_by = [recipient, sentAt] (compound) ──────────────
        MatrixCase {
            label: "[recipient, sentAt] / where=recipient IN[2] AND sentAt > floor",
            platform_allowed: "yes (GroupByCompound — `(In, range)` shape)",
            raw_where: where_recipient_in_sent_at_gt(),
            raw_order_by: Value::Null,
            mode: SumMode::GroupByCompound,
            limit: Some(100),
        },
        MatrixCase {
            label: "[recipient, sentAt] / where=recipient IN[2] AND sentAt==Y",
            platform_allowed: "no — `sentAt` must be range, not `==`",
            raw_where: where_recipient_in_sent_at_eq(),
            raw_order_by: Value::Null,
            mode: SumMode::GroupByCompound,
            limit: Some(100),
        },
    ];

    for case in &cases {
        let noproof_result = drive_sum_outcome(
            fixture,
            SUM_PROPERTY_NAME,
            case.raw_where.clone(),
            case.raw_order_by.clone(),
            case.mode,
            case.limit,
            false,
            platform_version,
        );
        let prove_result = drive_sum_outcome(
            fixture,
            SUM_PROPERTY_NAME,
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

/// Probe what's *actually* stored at `tip/recipient/recipient_050` and at
/// `tip/sentAt/sentAt_00050000` so a reviewer can confirm by reading
/// the live fixture which element types the two indexes produce.
///
/// This is the empirical answer to "why can't `byRecipient` use the same
/// `path=[..., "recipient"], Key(recipient_050)` shape as `bySentAt`?". The
/// shape only works when the resolved element is itself a sum-bearing
/// tree — for byRecipient (just `summable`, not `rangeSummable`) the
/// value tree is `Element::Tree` (a `NormalTree`) under the current
/// design, and `NormalTree::sum_value_or_default()` returns `0`, not the
/// aggregated amount. The optimization is structurally gated on the
/// index's `range_summable` flag for this exact reason.
fn probe_value_tree_types(fixture: &SumBenchFixture, _platform_version: &PlatformVersion) {
    use drive::drive::RootTree;
    use grovedb_path::SubtreePath;

    let contract_id = fixture.data_contract.id().to_buffer();
    let mid_recipient = recipient_id(RECIPIENT_COUNT / 2);
    let mid_sent_at = (fixture.row_count / 2).to_be_bytes();
    let cases: [(&'static str, &'static str, Vec<u8>); 2] = [
        ("byRecipient", "recipient", mid_recipient.to_vec()),
        ("bySentAt", "sentAt", mid_sent_at.to_vec()),
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
        match fixture
            .drive
            .grove
            .get(
                SubtreePath::from(parent.as_slice()),
                &val,
                None,
                grove_version,
            )
            .unwrap()
        {
            Ok(elem) => eprintln!(
                "[probe] {label}: tip/{prop}/{} → {} {{ sum_value_or_default: {}, debug: {:?} }}",
                hex_bytes(&val),
                element_variant_name(&elem),
                elem.sum_value_or_default(),
                elem
            ),
            Err(e) => eprintln!(
                "[probe] {label}: tip/{prop}/{} → grove.get error: {e:?}",
                hex_bytes(&val)
            ),
        }
    }

    // Probe the CHILDREN of each value tree to see how each one
    // contributes to the parent's sum_value_or_default. The
    // byRecipient value tree has children:
    //   - `[0]` (the ref-bucket SumTree where byRecipient's
    //     references live)
    //   - `sentAt` (the byRecipientTime continuation's
    //     property-name tree)
    // Are either of them wrapped in `Element::NonCounted(_)`-style
    // sum-skipping wrappers? That determines whether a hypothetical
    // "value tree is always a SumTree" rule would yield the correct
    // sum.
    let child_probes: Vec<(&'static str, &'static str, Vec<u8>, Vec<u8>)> = vec![
        (
            "byRecipient /[0] ref-bucket",
            "recipient",
            mid_recipient.to_vec(),
            vec![0u8],
        ),
        (
            "byRecipient /sentAt continuation",
            "recipient",
            mid_recipient.to_vec(),
            b"sentAt".to_vec(),
        ),
        (
            "bySentAt /[0] ref-bucket",
            "sentAt",
            mid_sent_at.to_vec(),
            vec![0u8],
        ),
    ];
    for (label, prop, val, child) in child_probes {
        let parent_owned: Vec<Vec<u8>> = vec![
            vec![RootTree::DataContractDocuments as u8],
            contract_id.to_vec(),
            vec![1u8],
            DOCUMENT_TYPE_NAME.as_bytes().to_vec(),
            prop.as_bytes().to_vec(),
            val,
        ];
        let parent: Vec<&[u8]> = parent_owned.iter().map(|v| v.as_slice()).collect();
        match fixture
            .drive
            .grove
            .get(
                SubtreePath::from(parent.as_slice()),
                &child,
                None,
                grove_version,
            )
            .unwrap()
        {
            Ok(elem) => eprintln!(
                "[probe-child] {label} → {} {{ sum_value_or_default: {}, debug: {:?} }}",
                element_variant_name(&elem),
                elem.sum_value_or_default(),
                elem
            ),
            Err(e) => eprintln!("[probe-child] {label} → grove.get error: {e:?}"),
        }
    }
}

/// Map a grovedb Element to a short human-readable variant name. The
/// match arms intentionally include every sum/count variant we expect
/// to encounter under the tip-jar's index layout — anything else falls
/// through to `"(other-variant)"` so the probe output flags it for a
/// human to look at rather than silently lying about the shape.
fn element_variant_name(e: &grovedb::Element) -> &'static str {
    use grovedb::Element;
    match e {
        Element::SumTree(_, _, _) => "SumTree",
        Element::ProvableSumTree(_, _, _) => "ProvableSumTree",
        Element::BigSumTree(_, _, _) => "BigSumTree",
        Element::CountTree(_, _, _) => "CountTree",
        Element::ProvableCountTree(_, _, _) => "ProvableCountTree",
        Element::CountSumTree(_, _, _, _) => "CountSumTree",
        Element::ProvableCountSumTree(_, _, _, _) => "ProvableCountSumTree",
        // grovedb PR 670: per-node count AND per-node sum committed
        // — distinct from `ProvableCountSumTree` (per-node count
        // only; sum at root). The bench will see these as
        // property-name trees of indexes that declare both
        // `rangeCountable: true` AND `rangeSummable: true`, and as
        // primary-key trees with both range flags set at the doctype.
        Element::ProvableCountProvableSumTree(_, _, _, _) => "ProvableCountProvableSumTree",
        Element::Tree(_, _) => "Tree (NormalTree)",
        Element::Item(_, _) => "Item",
        Element::SumItem(_, _) => "SumItem",
        Element::ItemWithSumItem(_, _, _) => "ItemWithSumItem",
        Element::Reference(_, _, _) => "Reference",
        // grovedb PR 670: a Reference that also carries an i64
        // sum-item contribution. The bench will see these under
        // every summable-index path when proofs are dumped (each
        // doc_id reference at `[index_path, value, 0, doc_id]` is a
        // ReferenceWithSumItem rather than a plain Reference).
        Element::ReferenceWithSumItem(_, _, _, _) => "ReferenceWithSumItem",
        _ => "(other-variant)",
    }
}

/// Decoded display of every `group_by = []` proof shape.
///
/// For each case, this:
/// 1. Re-runs the drive dispatcher to get the proof bytes.
/// 2. Reconstructs the **same `PathQuery`** the prover used (by
///    calling the matching builder on `DriveDocumentSumQuery` —
///    the single source of truth shared by prover + verifier).
/// 3. Runs the appropriate grovedb verifier
///    (`verify_query` for point-lookup / primary-key proofs,
///    `verify_aggregate_sum_query` for the range-aggregate
///    primitive) and prints the verified payload.
///
/// The output is structured so a reader can correlate each
/// proof's size with the path-query shape AND the merk elements
/// the proof signs, without parsing raw merk-proof bytes.
fn display_proofs(fixture: &SumBenchFixture, platform_version: &PlatformVersion) {
    let document_type = fixture
        .data_contract
        .document_type_for_name(DOCUMENT_TYPE_NAME)
        .expect("tip doc type");
    let _contract_id = fixture.data_contract.id().to_buffer();
    let recipients_2 = recipients_n(2);
    let recipients_100 = recipients_n(RECIPIENT_COUNT);
    let sent_ats_2 = first_n_sent_at_values(2);
    let mid_recipient = recipient_id(RECIPIENT_COUNT / 2);
    let mid_sent_at = fixture.row_count / 2;
    let range_floor = Value::U64(fixture.range_floor);

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
    //   query builder consumes (mirrors what `parse_sum_where_value`
    //   would produce on the dispatcher side)
    // - `shape`: which verifier primitive applies
    enum Shape {
        PrimaryKey,
        PointLookup,
        AggregateRange,
        /// Carrier-aggregate sum (`In` on the outer prefix property +
        /// `AggregateSumOnRange` on the index's terminator). Routes
        /// through
        /// [`DriveDocumentSumQuery::carrier_aggregate_sum_path_query_static`]
        /// for the verifier-side path-query rebuild and
        /// [`GroveDb::verify_aggregate_sum_query_per_key`] for the
        /// per-In-key sum extraction (grovedb PR #670).
        CarrierAggregate {
            limit: Option<u16>,
            left_to_right: bool,
        },
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
            label: "[] / where=recipient==X",
            raw_where: Value::Array(vec![clause(
                "recipient",
                "==",
                Value::Bytes(mid_recipient.to_vec()),
            )]),
            structured: vec![WhereClause {
                field: "recipient".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Bytes(mid_recipient.to_vec()),
            }],
            shape: Shape::PointLookup,
        },
        DisplayCase {
            label: "[] / where=sentAt==X",
            raw_where: Value::Array(vec![clause("sentAt", "==", Value::U64(mid_sent_at))]),
            structured: vec![WhereClause {
                field: "sentAt".to_string(),
                operator: WhereOperator::Equal,
                value: Value::U64(mid_sent_at),
            }],
            shape: Shape::PointLookup,
        },
        DisplayCase {
            label: "[] / where=recipient==X AND sentAt==Y",
            raw_where: Value::Array(vec![
                clause("recipient", "==", Value::Bytes(mid_recipient.to_vec())),
                clause("sentAt", "==", Value::U64(mid_sent_at)),
            ]),
            structured: vec![
                WhereClause {
                    field: "recipient".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Bytes(mid_recipient.to_vec()),
                },
                WhereClause {
                    field: "sentAt".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::U64(mid_sent_at),
                },
            ],
            shape: Shape::PointLookup,
        },
        DisplayCase {
            label: "[] / where=recipient IN[2]",
            raw_where: Value::Array(vec![clause(
                "recipient",
                "in",
                Value::Array(recipients_2.clone()),
            )]),
            structured: vec![WhereClause {
                field: "recipient".to_string(),
                operator: WhereOperator::In,
                value: Value::Array(recipients_2.clone()),
            }],
            shape: Shape::PointLookup,
        },
        DisplayCase {
            label: "[] / where=sentAt IN[2]",
            raw_where: Value::Array(vec![clause(
                "sentAt",
                "in",
                Value::Array(sent_ats_2.clone()),
            )]),
            structured: vec![WhereClause {
                field: "sentAt".to_string(),
                operator: WhereOperator::In,
                value: Value::Array(sent_ats_2.clone()),
            }],
            shape: Shape::PointLookup,
        },
        DisplayCase {
            label: "[] / where=sentAt > floor",
            raw_where: Value::Array(vec![clause("sentAt", ">", range_floor.clone())]),
            structured: vec![WhereClause {
                field: "sentAt".to_string(),
                operator: WhereOperator::GreaterThan,
                value: range_floor.clone(),
            }],
            shape: Shape::AggregateRange,
        },
        DisplayCase {
            label: "[] / where=recipient==X AND sentAt > floor",
            raw_where: Value::Array(vec![
                clause("recipient", "==", Value::Bytes(mid_recipient.to_vec())),
                clause("sentAt", ">", range_floor.clone()),
            ]),
            structured: vec![
                WhereClause {
                    field: "recipient".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Bytes(mid_recipient.to_vec()),
                },
                WhereClause {
                    field: "sentAt".to_string(),
                    operator: WhereOperator::GreaterThan,
                    value: range_floor.clone(),
                },
            ],
            shape: Shape::AggregateRange,
        },
        // Q9 — carrier-aggregate sum. Outer `In` over all 100
        // recipients + inner `AggregateSumOnRange` on `sentAt > floor`,
        // grouped by `[recipient, sentAt]` with `limit=100`. The
        // dispatcher routes this to `SumMode::GroupByCompound`; the
        // verifier-side path query is rebuilt via
        // `DriveDocumentSumQuery::carrier_aggregate_sum_path_query_static`
        // and the per-In-key sums are extracted via
        // `GroveDb::verify_aggregate_sum_query_per_key`.
        DisplayCase {
            label: "[recipient, sentAt] / where=recipient IN[100] AND sentAt > floor",
            raw_where: in_and_range_where_value(recipients_100.clone(), range_floor.clone()),
            structured: vec![
                WhereClause {
                    field: "recipient".to_string(),
                    operator: WhereOperator::In,
                    value: Value::Array(recipients_100.clone()),
                },
                WhereClause {
                    field: "sentAt".to_string(),
                    operator: WhereOperator::GreaterThan,
                    value: range_floor.clone(),
                },
            ],
            shape: Shape::CarrierAggregate {
                limit: Some(100),
                left_to_right: true,
            },
        },
    ];

    for case in cases {
        // 1. Get the proof bytes via the drive dispatcher.
        //
        // Carrier-aggregate shape uses `SumMode::GroupByIn` —
        // `(GroupByIn, has_range, has_in, prove) →
        // DocumentSumMode::RangeAggregateCarrierProof` per the
        // routing table in
        // `drive_document_sum_query/mode_detection/v0/mod.rs`.
        // `GroupByCompound` is reserved for the per-`(in_key,
        // range_key)` distinct walk (`RangeDistinctProof`), which
        // is a different proof shape that
        // `verify_aggregate_sum_query_per_key` (called below for
        // the carrier branch) wouldn't accept. Pinning the
        // GroupByIn mode here keeps the carrier-aggregate case's
        // prover/verifier in lock-step; every other case rides
        // the basic `Aggregate` mode.
        let (request_mode, request_limit) = match case.shape {
            Shape::CarrierAggregate { limit, .. } => (SumMode::GroupByIn, limit.map(|l| l as u32)),
            _ => (SumMode::Aggregate, None),
        };
        let make_request = || {
            sum_request(
                fixture,
                SUM_PROPERTY_NAME,
                case.raw_where.clone(),
                Value::Null,
                request_mode,
                request_limit,
                true,
            )
        };
        // Soft-skip when the prover errors (e.g. a fixture-layout
        // issue surfaces a `CorruptedData` from grovedb's
        // AggregateSumOnRange validator). Lets the rest of the report
        // print rather than panicking out of the entire display.
        let proof_bytes =
            match fixture
                .drive
                .execute_document_sum_request(make_request(), None, platform_version)
            {
                Ok(DocumentSumResponse::Proof(p)) => p,
                Ok(other) => panic!("display_proofs: expected Proof, got {other:?}"),
                Err(e) => {
                    eprintln!(
                        "\n[display] {label}\n  skipped — proof request errored: {e:?}",
                        label = case.label
                    );
                    continue;
                }
            };
        // Median-of-5 prover-side wall-clock for the Avg-time column
        // in the book chapter. Warmup happens inside `time_median`.
        let median = time_median(5, || {
            let _ =
                fixture
                    .drive
                    .execute_document_sum_request(make_request(), None, platform_version);
        });

        // 2. Rebuild the same PathQuery the prover used. The
        // primary-key form is the simple scalar-args static; the
        // point-lookup and aggregate-range forms route through the
        // `_static` wrappers which re-pick the covering index from
        // the document type before delegating to the instance method.
        let path_query: PathQuery = match case.shape {
            Shape::PrimaryKey => DriveDocumentSumQuery::primary_key_sum_path_query(
                fixture.data_contract.id().to_buffer(),
                document_type.name(),
            ),
            Shape::PointLookup => DriveDocumentSumQuery::point_lookup_sum_path_query_static(
                &fixture.data_contract,
                document_type,
                SUM_PROPERTY_NAME,
                &case.structured,
                platform_version,
            )
            .expect("point-lookup path query builds"),
            Shape::AggregateRange => DriveDocumentSumQuery::aggregate_sum_path_query_static(
                &fixture.data_contract,
                document_type,
                SUM_PROPERTY_NAME,
                &case.structured,
                platform_version,
            )
            .expect("aggregate-range path query builds"),
            Shape::CarrierAggregate {
                limit,
                left_to_right,
            } => DriveDocumentSumQuery::carrier_aggregate_sum_path_query_static(
                &fixture.data_contract,
                document_type,
                SUM_PROPERTY_NAME,
                &case.structured,
                limit,
                left_to_right,
                platform_version,
            )
            .expect("carrier-aggregate path query builds"),
        };

        eprintln!(
            "\n[display] {label}\n  proof_size: {bytes} bytes  median={us:.1} µs\n  path: {path}\n  items: {items}",
            label = case.label,
            bytes = proof_bytes.len(),
            us = median.as_secs_f64() * 1_000_000.0,
            path = display_segments(&path_query.path),
            items = display_query_items(&path_query.query.query.items),
        );

        // 3. Verify the proof and decode the verified payload.
        match case.shape {
            Shape::PrimaryKey | Shape::PointLookup => {
                match GroveDb::verify_query(
                    &proof_bytes,
                    &path_query,
                    &platform_version.drive.grove_version,
                ) {
                    Ok((root_hash, results)) => {
                        eprintln!("  verified: root_hash={}", hex_bytes(&root_hash));
                        for (path, key, elem) in &results {
                            eprintln!(
                                "    path={} key={} elem={}",
                                display_segments(path),
                                hex_bytes(key),
                                display_element(elem.as_ref()),
                            );
                        }
                    }
                    Err(e) => eprintln!("  verify_query error: {e:?}"),
                }

                // Also print the decoded proof AST for cross-reference
                // with the structured display in the book. PathQuery
                // proofs are bincode-encoded big-endian with no length
                // limit; mirror count's analog config.
                let bincode_config = bincode::config::standard()
                    .with_big_endian()
                    .with_no_limit();
                match bincode::decode_from_slice::<GroveDBProof, _>(&proof_bytes, bincode_config) {
                    Ok((decoded, _)) => eprintln!("  proof_ast:\n{decoded}"),
                    Err(e) => eprintln!("  proof deserialize error: {e:?}"),
                }
            }
            Shape::AggregateRange => {
                match GroveDb::verify_aggregate_sum_query(
                    &proof_bytes,
                    &path_query,
                    &platform_version.drive.grove_version,
                ) {
                    Ok((root_hash, sum)) => {
                        eprintln!("  verified: root_hash={} sum={sum}", hex_bytes(&root_hash));
                    }
                    Err(e) => eprintln!("  verify_aggregate_sum_query error: {e:?}"),
                }

                let bincode_config = bincode::config::standard()
                    .with_big_endian()
                    .with_no_limit();
                match bincode::decode_from_slice::<GroveDBProof, _>(&proof_bytes, bincode_config) {
                    Ok((decoded, _)) => eprintln!("  proof_ast:\n{decoded}"),
                    Err(e) => eprintln!("  proof deserialize error: {e:?}"),
                }
            }
            Shape::CarrierAggregate { .. } => {
                // Per-In-key aggregate sum entries; each pair binds
                // the serialized In-key bytes to its inner ASOR
                // aggregate sum. Same verifier surface count uses
                // for its carrier-ACOR equivalent.
                match GroveDb::verify_aggregate_sum_query_per_key(
                    &proof_bytes,
                    &path_query,
                    &platform_version.drive.grove_version,
                ) {
                    Ok((root_hash, entries)) => {
                        eprintln!(
                            "  verified: root_hash={} entries={}",
                            hex_bytes(&root_hash),
                            entries.len(),
                        );
                        for (in_key, sum) in &entries {
                            eprintln!("    in_key=0x{} sum={sum}", hex_bytes(in_key),);
                        }
                    }
                    Err(e) => {
                        eprintln!("  verify_aggregate_sum_query_per_key error: {e:?}")
                    }
                }

                let bincode_config = bincode::config::standard()
                    .with_big_endian()
                    .with_no_limit();
                match bincode::decode_from_slice::<GroveDBProof, _>(&proof_bytes, bincode_config) {
                    Ok((decoded, _)) => eprintln!("  proof_ast:\n{decoded}"),
                    Err(e) => eprintln!("  proof deserialize error: {e:?}"),
                }
            }
        }
    }
}

/// Compact path-segment display used by `display_proofs`. Each
/// segment is either UTF-8 (for property names) or raw bytes
/// (for serialized index values); render UTF-8 inline and fall
/// back to hex for non-UTF-8.
fn display_segments(path: &[Vec<u8>]) -> String {
    let parts: Vec<String> = path
        .iter()
        .map(|seg| match std::str::from_utf8(seg) {
            Ok(s) if s.chars().all(|c| !c.is_control()) => format!("{:?}", s),
            _ => format!("0x{}", hex_bytes(seg)),
        })
        .collect();
    format!("[{}]", parts.join(", "))
}

/// Render a list of `QueryItem`s in a compact form for the
/// `display_proofs` log lines.
fn display_query_items(items: &[grovedb::QueryItem]) -> String {
    let parts: Vec<String> = items
        .iter()
        .map(|item| match item {
            grovedb::QueryItem::Key(k) => format!("Key({})", display_segment_bytes(k)),
            grovedb::QueryItem::Range(r) => format!(
                "Range({}..{})",
                display_segment_bytes(&r.start),
                display_segment_bytes(&r.end)
            ),
            grovedb::QueryItem::RangeInclusive(r) => format!(
                "RangeInclusive({}..={})",
                display_segment_bytes(r.start()),
                display_segment_bytes(r.end())
            ),
            grovedb::QueryItem::RangeFull(_) => "RangeFull".to_string(),
            grovedb::QueryItem::RangeFrom(r) => {
                format!("RangeFrom({}..)", display_segment_bytes(&r.start))
            }
            grovedb::QueryItem::RangeTo(r) => {
                format!("RangeTo(..{})", display_segment_bytes(&r.end))
            }
            grovedb::QueryItem::RangeToInclusive(r) => {
                format!("RangeToInclusive(..={})", display_segment_bytes(&r.end))
            }
            grovedb::QueryItem::RangeAfter(r) => {
                format!("RangeAfter({}..)", display_segment_bytes(&r.start))
            }
            grovedb::QueryItem::RangeAfterTo(r) => format!(
                "RangeAfterTo({}..{})",
                display_segment_bytes(&r.start),
                display_segment_bytes(&r.end)
            ),
            grovedb::QueryItem::RangeAfterToInclusive(r) => format!(
                "RangeAfterToInclusive({}..={})",
                display_segment_bytes(r.start()),
                display_segment_bytes(r.end())
            ),
            other => format!("{:?}", other),
        })
        .collect();
    format!("[{}]", parts.join(", "))
}

/// UTF-8 if it's printable, otherwise hex — same convention used
/// across the display helpers.
fn display_segment_bytes(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) if s.chars().all(|c| !c.is_control()) => format!("{:?}", s),
        _ => format!("0x{}", hex_bytes(bytes)),
    }
}

/// Render the verified-element variant + its sum contribution for
/// `display_proofs`. Mirrors count's `display_element` shape.
fn display_element(elem: Option<&grovedb::Element>) -> String {
    match elem {
        None => "(absent)".to_string(),
        Some(e) => format!(
            "{} {{ sum_value_or_default: {} }}",
            element_variant_name(e),
            e.sum_value_or_default()
        ),
    }
}

/// Compact hex helper used by `display_segments` / `display_proofs`.
fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Convenience helper for the matrix runner: run one sum request
/// through the drive dispatcher and format the outcome as a short
/// string (success → describing the response shape and size; error
/// → the truncated error message). Keeps `report_group_by_matrix`'s
/// per-case body readable.
#[allow(clippy::too_many_arguments)]
fn drive_sum_outcome(
    fixture: &SumBenchFixture,
    sum_property: &str,
    raw_where: Value,
    raw_order_by: Value,
    mode: SumMode,
    limit: Option<u32>,
    prove: bool,
    platform_version: &PlatformVersion,
) -> String {
    let request = sum_request(
        fixture,
        sum_property,
        raw_where,
        raw_order_by,
        mode,
        limit,
        prove,
    );
    match fixture
        .drive
        .execute_document_sum_request(request, None, platform_version)
    {
        Ok(DocumentSumResponse::Aggregate(s)) => format!("Aggregate({s})"),
        Ok(DocumentSumResponse::Entries(entries)) => {
            let summed: i64 = entries.iter().filter_map(|e| e.sum).sum();
            format!("Entries(len={}, sum={})", entries.len(), summed)
        }
        Ok(DocumentSumResponse::Proof(p)) => format!("Proof({} bytes)", p.len()),
        Err(e) => {
            let msg = e.to_string();
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

/// First N recipients by id — convenience for matrix cases that need
/// a small In array (2-3 recipients) rather than the full 100 used by
/// the criterion benches.
fn recipients_n(n: u64) -> Vec<Value> {
    (0..n)
        .map(|r| Value::Bytes(recipient_id(r).to_vec()))
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn sum_request<'a>(
    fixture: &'a SumBenchFixture,
    sum_property: &str,
    raw_where_value: Value,
    raw_order_by_value: Value,
    mode: SumMode,
    limit: Option<u32>,
    prove: bool,
) -> DocumentSumRequest<'a> {
    use drive::query::drive_document_sum_query::drive_dispatcher::{
        order_clauses_from_value, where_clauses_from_value,
    };

    let document_type = fixture
        .data_contract
        .document_type_for_name(DOCUMENT_TYPE_NAME)
        .expect("expected tip document type");

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

    DocumentSumRequest {
        contract: &fixture.data_contract,
        document_type,
        sum_property: sum_property.to_string(),
        where_clauses,
        order_clauses,
        mode,
        limit,
        prove,
        drive_config: &fixture.drive_config,
    }
}

fn recipient_in_where_value(recipients: Vec<Value>) -> Value {
    Value::Array(vec![Value::Array(vec![
        Value::Text("recipient".to_string()),
        Value::Text("in".to_string()),
        Value::Array(recipients),
    ])])
}

fn sent_at_in_where_value(sent_ats: Vec<Value>) -> Value {
    Value::Array(vec![Value::Array(vec![
        Value::Text("sentAt".to_string()),
        Value::Text("in".to_string()),
        Value::Array(sent_ats),
    ])])
}

/// First N sentAt values — same naming convention as `populate_fixture`
/// (`sentAt = row`, monotonically increasing), which guarantees these
/// values exist in the fixture so the proof actually resolves
/// 100 present branches (not absent ones, which would be omitted
/// from the proof's emitted-elements stream and shrink the proof
/// trivially).
fn first_n_sent_at_values(n: u64) -> Vec<Value> {
    (0..n).map(Value::U64).collect()
}

fn in_and_range_where_value(recipients: Vec<Value>, range_floor: Value) -> Value {
    Value::Array(vec![
        Value::Array(vec![
            Value::Text("recipient".to_string()),
            Value::Text("in".to_string()),
            Value::Array(recipients),
        ]),
        Value::Array(vec![
            Value::Text("sentAt".to_string()),
            Value::Text(">".to_string()),
            range_floor,
        ]),
    ])
}

fn all_recipient_values() -> Vec<Value> {
    (0..RECIPIENT_COUNT)
        .map(|r| Value::Bytes(recipient_id(r).to_vec()))
        .collect()
}

/// Deterministic 32-byte recipient id derived from a small index.
/// First 8 bytes are the big-endian u64 of `n`; remaining bytes are
/// the bitwise NOT of those 8 bytes, then zero-padded. This gives:
/// - distinct, monotonically-sortable ids for n ∈ [0, 2^64),
/// - reproducibility across bench runs and machines,
/// - a non-trivial second-half pattern so collisions can't sneak in
///   from a partial-prefix comparison.
fn recipient_id(n: u64) -> [u8; 32] {
    let mut id = [0u8; 32];
    id[..8].copy_from_slice(&n.to_be_bytes());
    for (i, byte) in n.to_be_bytes().iter().enumerate() {
        id[8 + i] = !byte;
    }
    id
}

/// Deterministic document id derived from row index. Mirrors
/// `document_count_worst_case::document_id`'s construction (BE row
/// number in the high half, bitwise-NOT of it in the next 8 bytes,
/// zeroed tail) so the two benches' primary-key layouts are
/// shape-identical.
fn document_id(row: u64) -> [u8; 32] {
    let mut id = [0u8; 32];
    let document_number = row + 1;
    id[..8].copy_from_slice(&document_number.to_be_bytes());
    id[8..16].copy_from_slice(&(!document_number).to_be_bytes());
    id
}

fn row_count() -> u64 {
    env_u64("DASH_PLATFORM_SUM_BENCH_ROWS").unwrap_or(DEFAULT_ROW_COUNT)
}

fn batch_size() -> u64 {
    env_u64("DASH_PLATFORM_SUM_BENCH_BATCH_SIZE").unwrap_or(DEFAULT_BATCH_SIZE)
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
    if let Ok(path) = env::var("DASH_PLATFORM_SUM_BENCH_DB") {
        return PathBuf::from(path);
    }

    env::temp_dir().join(format!(
        "dash-platform-document-sum-bench-v{FIXTURE_SCHEMA_VERSION}-rows-{row_count}"
    ))
}

fn fixture_marker(row_count: u64) -> String {
    let protocol_version = PlatformVersion::latest().protocol_version;
    format!(
        "schema_version={FIXTURE_SCHEMA_VERSION}\nprotocol_version={protocol_version}\nrows={row_count}\nrecipients={RECIPIENT_COUNT}\n"
    )
}

criterion_group!(sum_query_worst_cases, document_sum_worst_case);
criterion_main!(sum_query_worst_cases);
