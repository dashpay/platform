//! Worst-case benchmarks for **average-query** path shapes on the
//! grades contract. Average queries return `(count, sum)` from a single
//! root-hash-committed grovedb traversal; the client computes
//! `avg = sum / count`. Companion to
//! [`document_count_worst_case.rs`](document_count_worst_case.rs) and
//! [`document_sum_worst_case.rs`](document_sum_worst_case.rs), feeding
//! the [average-index-examples chapter]
//! (../../../../book/src/drive/average-index-examples.md) the same way
//! those benches feed their respective chapters.
//!
//! The fixture uses the live grades contract at
//! [`packages/rs-drive/tests/supporting_files/contract/grades/grades-contract.json`](../tests/supporting_files/contract/grades/grades-contract.json) —
//! NOT inlined as a `platform_value!` literal. (The sum bench inlines
//! its tip-jar schema; the grades schema is the source of truth in JSON
//! because the chapter quotes the file directly.)
//!
//! Average queries don't have a dedicated `DriveDocumentAverageQuery`
//! type — averages are read from `CountSumTree` elements (count + sum
//! per merk node) and `ProvableCountProvableSumTree` (PCPS, count + sum
//! per *internal* merk node, supporting `AggregateCountAndSumOnRange`).
//! This bench composes them via:
//! - direct `grove.get_proved_path_query` calls for the point-lookup
//!   shapes (Q1-Q4 and Q6, the CountSumTree-carrier), since those
//!   resolve a `CountSumTree` element terminator that `GroveDb::verify_query`
//!   already exposes — the verified `Element::CountSumTree(_, count,
//!   sum, _)` holds both metrics
//! - the new leaf-PCPS executor `DriveDocumentSumQuery::execute_aggregate_count_and_sum_with_proof`
//!   for Q5 (`AggregateCountAndSumOnRange` against the byClassSemester
//!   PCPS continuation)
//! - the existing PCPS-carrier `DriveDocumentSumQuery::execute_carrier_aggregate_count_and_sum_with_proof`
//!   for Q7 (per-class PCPS carrier)
//!
//! Environment knobs (mirror the sum bench's, with the `AVG` prefix):
//! - `DASH_PLATFORM_AVG_BENCH_ROWS`: row count; defaults to 10 000
//!   (= 100 students × 10 classes × 10 semesters; every cell is
//!   populated).
//! - `DASH_PLATFORM_AVG_BENCH_DB`: fixture directory.
//! - `DASH_PLATFORM_AVG_BENCH_REBUILD=1`: nuke the fixture before
//!   building.
//! - `DASH_PLATFORM_AVG_BENCH_BATCH_SIZE`: inserts per commit;
//!   defaults to 2 500.

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::DataContract;
use dpp::document::{Document, DocumentV0};
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::tests::json_document::json_document_to_contract;
use dpp::version::PlatformVersion;
use drive::config::DriveConfig;
use drive::drive::Drive;
use drive::query::{DriveDocumentSumQuery, WhereClause, WhereOperator};
use drive::util::object_size_info::DocumentInfo::DocumentRefInfo;
use drive::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use drive::util::storage_flags::StorageFlags;
use grovedb::operations::proof::GroveDBProof;
use grovedb::{GroveDb, PathQuery, Query, QueryItem, SizedQuery};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

// ---------------------------------------------------------------------
// Bench-level constants (mirror sum bench naming, AVG prefix).
// ---------------------------------------------------------------------

/// Bumped when the on-disk fixture layout changes (so cached fixtures
/// are invalidated automatically on schema-shape changes).
const FIXTURE_SCHEMA_VERSION: u32 = 3;
/// 500 students × 10 classes × 10 semesters = 50 000 grades total.
const DEFAULT_ROW_COUNT: u64 = 50_000;
const DEFAULT_BATCH_SIZE: u64 = 5_000;
const STUDENT_COUNT: u64 = 500;
const CLASS_COUNT: u64 = 10;
const SEMESTER_COUNT: u64 = 10;

/// Semantic class names (instead of `PHYS101`..`CLASS09`) so the
/// chapter's worked-example output reads like a real transcript.
/// Difficulty profile mapping is hand-tuned in [`class_profile`] to
/// produce a real-data-shaped spread across the carrier's per-class
/// averages (PHYS101 ~ 60, ARTS101 ~ 88, etc.) rather than the
/// pathologically uniform `≈50` cluster the previous formula produced.
const CLASS_NAMES: [&str; CLASS_COUNT as usize] = [
    "PHYS101", // 0 — hardest, widest spread
    "CHEM101", // 1 — moderately hard
    "CALC201", // 2 — also hard
    "ENGL101", // 3 — easy, narrow spread
    "HIST101", // 4 — moderate
    "BIOL101", // 5 — moderate
    "ARTS101", // 6 — easiest
    "COMP101", // 7 — moderate-wide spread (skill amplified)
    "MUSC101", // 8 — easy
    "SOCI101", // 9 — easy
];

/// Per-class `(mean, spread)` — hand-tuned to produce realistic
/// per-class averages on the carrier-aggregate result. `mean` is the
/// raw class baseline; `spread` is the multiplier that scales student
/// skill effect, so harder classes (wider spread) amplify skill
/// differences (top students earn proportionally more, struggling
/// students suffer proportionally more) and easier classes compress
/// the gap.
const fn class_profile(class_idx: u64) -> (i64, i64) {
    match class_idx {
        0 => (60, 12), // PHYS101 — hard physics
        1 => (65, 10), // CHEM101 — moderate chem
        2 => (58, 13), // CALC201 — hardest math
        3 => (85, 5),  // ENGL101 — easy english
        4 => (78, 8),  // HIST101 — moderate
        5 => (72, 9),  // BIOL101 — moderate
        6 => (88, 4),  // ARTS101 — easiest
        7 => (75, 9),  // COMP101 — moderate
        8 => (82, 6),  // MUSC101 — easy
        9 => (80, 6),  // SOCI101 — easy social
        _ => (70, 8),  // defensive default if CLASS_COUNT ever grows
    }
}

/// Per-class **enrollment rate** as a percent — what fraction of
/// `(student, semester)` slots a class fills. Real transcripts have
/// uneven enrollment: hard classes have fewer students; required
/// classes have everyone; easy classes are popular. The bench's
/// enrollment matrix is generated deterministically by
/// [`is_enrolled`] which hashes `(student, class, semester)` and
/// keeps the grade iff the hash bucket falls under this percentage.
///
/// Expected per-(student, semester) class load ≈ Σ popularities / 100
/// = (30+45+25+100+70+60+90+55+85+70) / 100 ≈ **6.3 classes per
/// semester**. Across 10 semesters × 500 students that's about
/// 31 500 grades total — versus the 50 000 the previous "everyone
/// takes everything" fixture produced.
const CLASS_POPULARITY: [u8; CLASS_COUNT as usize] = [
    30,  // 0 PHYS101 — hard physics, only ~30% enroll
    45,  // 1 CHEM101 — moderately popular
    25,  // 2 CALC201 — hardest math, smallest cohort
    100, // 3 ENGL101 — required for everyone every semester
    70,  // 4 HIST101 — common humanities elective
    60,  // 5 BIOL101 — moderately popular
    90,  // 6 ARTS101 — very popular (easy GPA boost)
    55,  // 7 COMP101 — moderately popular
    85,  // 8 MUSC101 — popular elective
    70,  // 9 SOCI101 — common social-science elective
];

/// Deterministic enrollment decision for one `(student, class,
/// semester)` triple. Hashes the triple via a multiplicative mix and
/// compares the result mod 100 against the class's popularity
/// percentage. Result is reproducible across bench runs (no PRNG
/// state), independent across triples (no spatial correlation), and
/// produces the per-class enrollment rates in [`CLASS_POPULARITY`]
/// in expectation.
///
/// Special-cased: ENGL101 (popularity 100) returns `true`
/// unconditionally — semantically "everyone takes English every
/// semester," and we want that to be guaranteed rather than
/// statistically-near-100%.
fn is_enrolled(student_idx: u64, class_idx: u64, semester_idx: u64) -> bool {
    let pop = CLASS_POPULARITY[class_idx as usize];
    if pop >= 100 {
        return true;
    }
    let key = student_idx
        .wrapping_mul(0xD6E8FEB86659FD93)
        .wrapping_add(class_idx.wrapping_mul(0xCF4A66B7FB39CF4D))
        .wrapping_add(semester_idx.wrapping_mul(0x9E3779B97F4A7C15));
    // Extract 8 bits (0..255) and scale to 0..99 via `× 100 / 256`.
    // This gives a uniform `(bucket < pop) == accept` mapping with no
    // edge bias — earlier versions used `(bits) % 100` on a 7-bit
    // value, which double-counted the 0..27 range due to wrap-around
    // and inflated low-popularity classes by 10–15 percentage points.
    let bucket = (((key >> 11) & 0xFF) as u32 * 100 / 256) as u8;
    bucket < pop
}

/// Per-student inherent skill, derived deterministically from the
/// student index via a cheap FNV-1a-style hash. Returns values
/// approximately in `[-25, +15]` with a bell-shaped distribution
/// centered just below 0 — i.e. most students are average, a few are
/// excellent, a few struggle. Sum-of-3-uniforms is the standard cheap
/// approximation for `N(0, σ²)` via the central limit theorem.
fn student_skill(student_idx: u64) -> i64 {
    // 64-bit FNV-1a hash of the student index.
    let mut h: u64 = 0xcbf29ce484222325;
    for b in student_idx.to_le_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    // Three independent uniforms in [-128, 127], averaged → ≈ N(0, 43²).
    let u1 = (h & 0xff) as i64 - 128;
    let u2 = ((h >> 8) & 0xff) as i64 - 128;
    let u3 = ((h >> 16) & 0xff) as i64 - 128;
    // Scaled so most students land in [-10, +10] with the tails reaching
    // [-25, +15] — slightly skewed negative so the average is "average,
    // not perfect."
    (u1 + u2 + u3) / 11 - 3
}

/// Deterministic per-grade noise in [-5, +5]. Adds the kind of
/// per-semester variation real grade data exhibits (good days, bad
/// days, slightly-different exam difficulty) so even a single
/// `(student, class)` pair has nontrivial semester-to-semester
/// variation.
fn grade_noise(student_idx: u64, class_idx: u64, semester_idx: u64) -> i64 {
    let key = student_idx.wrapping_mul(0x9E3779B97F4A7C15)
        ^ class_idx.wrapping_mul(0xBF58476D1CE4E5B9)
        ^ semester_idx.wrapping_mul(0x94D049BB133111EB);
    ((key >> 5) & 0xf) as i64 - 7
}

/// Compute a single grade. Combines class baseline, student skill
/// (amplified by class spread — harder classes magnify ability
/// gaps), and per-grade noise; clamps to [0, 100]. Deterministic in
/// `(student_idx, class_idx, semester_idx)`.
fn compute_score(student_idx: u64, class_idx: u64, semester_idx: u64) -> i64 {
    let (class_mean, spread) = class_profile(class_idx);
    let skill = student_skill(student_idx);
    let noise = grade_noise(student_idx, class_idx, semester_idx);
    // Skill is scaled by the class's spread so PHYS101 (spread=12)
    // amplifies a +10-skill student to +15, but ARTS101 (spread=4)
    // only adds +5. The denominator (8) anchors the scaling so a
    // "moderate" class (spread=8) leaves skill unchanged.
    let skill_effect = skill * spread / 8;
    (class_mean + skill_effect + noise).clamp(0, 100)
}
const DOCUMENT_TYPE_NAME: &str = "grade";
const SUM_PROPERTY_NAME: &str = "score";
const READY_MARKER: &str = ".document-average-worst-case-ready";

// ---------------------------------------------------------------------
// Fixture loader + populator.
// ---------------------------------------------------------------------

struct AvgBenchFixture {
    drive: Drive,
    data_contract: DataContract,
    #[allow(dead_code)]
    drive_config: DriveConfig,
    /// Maximum possible `(student, class, semester)` triples the bench
    /// walked. Used by [`fixture_marker`] / [`fixture_path`] so cached
    /// fixtures with the same triple count are reused across runs.
    #[allow(dead_code)]
    row_count: u64,
    /// **Actual number of grade documents inserted** after the
    /// `is_enrolled` filter dropped non-enrolled triples. This is the
    /// load-bearing number for Criterion throughput and the bench's
    /// `[proof-size]` headers — without it the published numbers would
    /// overstate the dataset size by ~30 % (the walked triple count vs.
    /// the actually-inserted grade count). Set by [`populate_fixture`].
    inserted_count: u64,
    /// Semester range floor used by the Q5 / Q7 range proofs.
    /// `semester > 20204` covers semesters 20205..20209 = exactly half
    /// the semester range. Predictable count + sum targets in the
    /// chapter's verified-result blocks.
    range_floor: u64,
}

impl AvgBenchFixture {
    fn load_or_create() -> Self {
        let row_count = row_count();
        let fixture_path = fixture_path(row_count);
        let rebuild = env_flag("DASH_PLATFORM_AVG_BENCH_REBUILD");
        let ready_marker = fixture_path.join(READY_MARKER);
        let expected_marker = fixture_marker(row_count);

        if rebuild && fixture_path.exists() {
            fs::remove_dir_all(&fixture_path)
                .expect("expected to remove old average bench fixture");
        }

        let data_contract = grades_contract();
        let drive_config = DriveConfig::default();

        if ready_marker.exists()
            && fs::read_to_string(&ready_marker)
                .expect("expected to read average bench fixture marker")
                == expected_marker
        {
            eprintln!(
                "reusing document-average fixture at {} with {} rows",
                fixture_path.display(),
                row_count
            );
            let (drive, _) = Drive::open(&fixture_path, Some(drive_config.clone()))
                .expect("expected to open existing average bench fixture");
            // Recount inserted grades by re-walking `is_enrolled`. The
            // function is deterministic and constant-time, so re-counting
            // 50 000 walks is a few microseconds — cheaper than persisting
            // the count to the marker file.
            let inserted_count = recount_enrolled(row_count);
            return Self::new(
                drive,
                data_contract,
                drive_config,
                row_count,
                inserted_count,
            );
        }

        if fixture_path.exists() {
            fs::remove_dir_all(&fixture_path)
                .expect("expected to remove incomplete average bench fixture");
        }
        fs::create_dir_all(&fixture_path).expect("expected to create average bench fixture dir");

        eprintln!(
            "building document-average fixture at {} with {} rows",
            fixture_path.display(),
            row_count
        );

        let started = Instant::now();
        let platform_version = PlatformVersion::latest();
        let (drive, _) = Drive::open(&fixture_path, Some(drive_config.clone()))
            .expect("expected to open new average bench fixture");

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
            .expect("expected to apply grades contract");

        let inserted_count = populate_fixture(&drive, &data_contract, row_count, platform_version);
        fs::write(&ready_marker, expected_marker)
            .expect("expected to mark average bench fixture ready");

        eprintln!(
            "built document-average fixture with {} actual grades \
             (from {} possible triples) in {:.2?}",
            inserted_count,
            row_count,
            started.elapsed()
        );

        Self::new(
            drive,
            data_contract,
            drive_config,
            row_count,
            inserted_count,
        )
    }

    fn new(
        drive: Drive,
        data_contract: DataContract,
        drive_config: DriveConfig,
        row_count: u64,
        inserted_count: u64,
    ) -> Self {
        // Semester floor: 20204. With semesters laid out as 20200..20209,
        // `semester > 20204` covers 20205..20209 = 5 of 10 semesters
        // (half the timeline). For a class IN [10 classes] over those
        // 5 semesters: 10 × 100 students × 5 semesters = 5 000 docs per
        // class × 10 classes carrier = 50 000 max docs touched, but each
        // class's inner aggregate touches only 100 students × 5 semesters
        // = 500 grades.
        let range_floor = 20204;

        Self {
            drive,
            data_contract,
            drive_config,
            row_count,
            inserted_count,
            range_floor,
        }
    }
}

/// Recompute the number of `(student, class, semester)` triples that
/// `is_enrolled` accepts, without touching the populated grovedb.
/// Used by the cache-reuse path on fixture load so the bench knows
/// the actual stored grade count even when it doesn't repopulate.
fn recount_enrolled(row_count: u64) -> u64 {
    let row_total = row_count.min(STUDENT_COUNT * CLASS_COUNT * SEMESTER_COUNT);
    (0..row_total)
        .filter(|row| {
            let student_idx = row % STUDENT_COUNT;
            let class_idx = (row / STUDENT_COUNT) % CLASS_COUNT;
            let semester_idx = row / (STUDENT_COUNT * CLASS_COUNT);
            is_enrolled(student_idx, class_idx, semester_idx)
        })
        .count() as u64
}

/// Load the grades contract from disk. The schema is the source of
/// truth in JSON form so the chapter's quoted `jsonc` block can stay
/// in sync with what the bench actually loads.
fn grades_contract() -> DataContract {
    let platform_version = PlatformVersion::latest();
    json_document_to_contract(
        "tests/supporting_files/contract/grades/grades-contract.json",
        false,
        platform_version,
    )
    .expect("expected to parse grades contract")
}

/// Deterministic insert schedule. Walks every `(student, class,
/// semester)` triple (500 × 10 × 10 = 50 000 possible triples) and
/// filters via [`is_enrolled`]; only enrolled triples produce a
/// grade document. Expected actual grade count ≈ 31 500 (varies
/// slightly by the hash distribution).
///
/// Score model (see [`compute_score`] for the exact formula):
///
/// ```text
/// score = clamp(class_mean[class_idx]
///             + student_skill[student_idx] × class_spread[class_idx] / 8
///             + grade_noise(student_idx, class_idx, semester_idx),
///             0, 100)
/// ```
///
/// Class profiles are hand-tuned (see [`class_profile`]) to look like
/// real transcripts: hard classes (PHYS101, CALC201) have low means
/// and wide spreads that amplify ability gaps; easy classes (ARTS101,
/// MUSC101) cluster scores near 85–90 with narrow spreads. Student
/// skill is a deterministic hash-based bell-shaped distribution
/// centered just below average. Per-grade noise covers the
/// semester-to-semester "good day / bad day" variation.
///
/// The result: per-class averages span ≈ 58 (CALC201) to ≈ 88
/// (ARTS101), and the per-student GPA spans ≈ 30 to ≈ 95 across the
/// 500-student cohort — a realistic spread for a chapter's worked
/// examples, not the pathologically uniform `≈50` cluster the prior
/// modular-arithmetic formula produced.
///
/// Semester layout: `semester_idx ∈ [0, 10)` → semester code
/// `20200 + semester_idx` = `20200..20209`. The chapter's queries use
/// `20204` as the range floor for Q5/Q7 ("trend"); `20204` was chosen
/// because it splits the timeline in half (5 below, 5 above) yielding
/// the cleanest reference numbers.
fn populate_fixture(
    drive: &Drive,
    data_contract: &DataContract,
    row_count: u64,
    platform_version: &PlatformVersion,
) -> u64 {
    let document_type = data_contract
        .document_type_for_name(DOCUMENT_TYPE_NAME)
        .expect("expected grade document type");
    let batch_size = batch_size();

    // Pre-compute the deterministic student / instructor id tables
    // once so each insert is one BTreeMap fill + one grove call.
    let students: Vec<[u8; 32]> = (0..STUDENT_COUNT).map(student_id_bytes).collect();
    let instructors: Vec<[u8; 32]> = (0..CLASS_COUNT).map(instructor_id_bytes).collect();

    // Stable insert order: walk every `(student, class, semester)`
    // triple in row order, but only emit a grade if [`is_enrolled`]
    // returns true. Realistic transcripts don't have everyone taking
    // every class every semester — popular electives have ~90%
    // enrollment, hard math classes have ~25%, English is required at
    // 100%. The exact per-class enrollment rates are in
    // [`CLASS_POPULARITY`].
    let mut next_row = 0;
    let row_total = row_count.min(STUDENT_COUNT * CLASS_COUNT * SEMESTER_COUNT);
    let mut inserted: u64 = 0;
    let mut last_report: u64 = 0;
    while next_row < row_total {
        let end_row = (next_row + batch_size).min(row_total);
        let transaction = drive.grove.start_transaction();

        for row in next_row..end_row {
            let student_idx = row % STUDENT_COUNT;
            let class_idx = (row / STUDENT_COUNT) % CLASS_COUNT;
            let semester_idx = row / (STUDENT_COUNT * CLASS_COUNT);
            if !is_enrolled(student_idx, class_idx, semester_idx) {
                continue;
            }
            insert_grade_document(
                drive,
                data_contract,
                document_type,
                row,
                students[student_idx as usize],
                class_idx,
                semester_idx,
                instructors[class_idx as usize],
                Some(&transaction),
                platform_version,
            );
            inserted += 1;
        }

        drive
            .grove
            .commit_transaction(transaction)
            .value
            .expect("expected average bench insert transaction to commit");

        next_row = end_row;
        // Report progress on every batch boundary in terms of triples
        // walked + actual grades inserted (the two diverge under the
        // enrollment filter, so the chapter's reproducibility narrative
        // can cite both).
        if next_row == row_total || inserted - last_report >= 2_500 {
            eprintln!(
                "inserted {inserted} grades after walking {next_row}/{row_total} (student, class, semester) triples"
            );
            last_report = inserted;
        }
    }
    eprintln!("populated {inserted} actual grade documents (of {row_total} possible triples)");
    inserted
}

#[allow(clippy::too_many_arguments)]
fn insert_grade_document(
    drive: &Drive,
    data_contract: &DataContract,
    document_type: dpp::data_contract::document_type::DocumentTypeRef,
    row: u64,
    student: [u8; 32],
    class_idx: u64,
    semester_idx: u64,
    instructor: [u8; 32],
    transaction: grovedb::TransactionArg,
    platform_version: &PlatformVersion,
) {
    let class_name = CLASS_NAMES[class_idx as usize].to_string();
    let semester = 20200 + semester_idx; // 20200..20209
    let student_idx = row % STUDENT_COUNT;
    // Score model — class baseline + student skill (amplified by
    // class difficulty) + per-grade noise. See [`compute_score`]
    // for the per-axis breakdown.
    let score = compute_score(student_idx, class_idx, semester_idx);

    let mut properties = BTreeMap::new();
    properties.insert("student".to_string(), Value::Bytes(student.to_vec()));
    properties.insert("class".to_string(), Value::Text(class_name));
    properties.insert("semester".to_string(), Value::I64(semester as i64));
    properties.insert("score".to_string(), Value::I64(score));
    properties.insert("instructor".to_string(), Value::Bytes(instructor.to_vec()));

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
        .expect("expected to insert average bench grade document");
}

// ---------------------------------------------------------------------
// Criterion entry point.
// ---------------------------------------------------------------------

fn document_average_worst_case(c: &mut Criterion) {
    let fixture = AvgBenchFixture::load_or_create();
    let platform_version = PlatformVersion::latest();

    // One-shot proof-size + median-µs report for the four headline
    // shapes (carrier-of-CountSumTree, leaf-PCPS, PCPS-carrier with
    // limit, CountSumTree-carrier for per-student-per-semester). Same
    // role as the sum bench's `report_proof_sizes`.
    report_proof_sizes(&fixture, platform_version);

    // Drive-side empirical matrix: what each `(group_by, where_shape)`
    // combination does on this contract — succeeds with a proof,
    // succeeds without a proof, or errors out. Same role as sum's
    // `report_group_by_matrix`. Average bench uses raw grovedb path
    // queries directly (no dispatcher) so the matrix is light — but
    // it still walks every Q1-Q7 shape and reports verified payloads.
    report_group_by_matrix(&fixture, platform_version);

    // Per-query proof AST + verified `(count, sum, avg)` for Q1-Q7.
    // This is the load-bearing output for backfilling the chapter:
    // each block is pasted byte-for-byte into the chapter's `<details>`
    // proof-display block.
    display_proofs(&fixture, platform_version);

    // Probe a representative per-index value-tree node to confirm
    // every index's terminator is the expected variant. Same role
    // as sum/count `probe_value_tree_types`.
    probe_value_tree_types(&fixture, platform_version);

    // Criterion timing for the four headline shapes.
    let mut group = c.benchmark_group("document_average_worst_case");
    group.sample_size(10);
    // Throughput uses the actually-inserted grade count, not the walked
    // triple count — the enrollment filter discards ~30 % of triples,
    // so reporting `row_count` would overstate the dataset's size.
    group.throughput(criterion::Throughput::Elements(fixture.inserted_count));

    let document_type = fixture
        .data_contract
        .document_type_for_name(DOCUMENT_TYPE_NAME)
        .expect("grade doc type");

    // Bench 1: carrier-of-CountSumTree per-class `(count, sum)`.
    // `class IN [10 classes]` → 10 CountSumTree branches, one per
    // class. `verify_query` reads `CountSumTree(_, count, sum, _)`
    // off each. No range; just 10 outer Key lookups.
    let all_classes: Vec<String> = (0..CLASS_COUNT).map(class_name).collect();
    group.bench_function("group_by_class_in_proof_10_countsum_branches", |b| {
        let class_keys: Vec<Vec<u8>> = all_classes.iter().map(|c| c.as_bytes().to_vec()).collect();
        b.iter_batched(
            || {
                build_class_in_path_query(
                    fixture.data_contract.id().to_buffer(),
                    DOCUMENT_TYPE_NAME,
                    &class_keys,
                )
            },
            |path_query| {
                let proof = fixture
                    .drive
                    .grove
                    .get_proved_path_query(
                        &path_query,
                        None,
                        None,
                        &platform_version.drive.grove_version,
                    )
                    .value
                    .expect("class-In proof");
                black_box(proof)
            },
            BatchSize::SmallInput,
        );
    });

    // Bench 2: leaf-PCPS Q5 — `AggregateCountAndSumOnRange` against
    // byClassSemester for `class==PHYS101 AND semester > floor`.
    let semester_floor_value = Value::I64(fixture.range_floor as i64);
    let q5_clauses = vec![
        WhereClause {
            field: "class".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("PHYS101".to_string()),
        },
        WhereClause {
            field: "semester".to_string(),
            operator: WhereOperator::GreaterThan,
            value: semester_floor_value.clone(),
        },
    ];
    let by_class_semester_index = document_type
        .indexes()
        .get("byClassSemester")
        .expect("byClassSemester index");
    group.bench_function(
        "aggregate_count_and_sum_class_math_semester_range_proof",
        |b| {
            let q = DriveDocumentSumQuery {
                document_type,
                contract_id: fixture.data_contract.id().to_buffer(),
                document_type_name: DOCUMENT_TYPE_NAME.to_string(),
                index: by_class_semester_index,
                where_clauses: q5_clauses.clone(),
                sum_property: SUM_PROPERTY_NAME.to_string(),
            };
            b.iter(|| {
                let proof = q
                    .execute_aggregate_count_and_sum_with_proof(
                        &fixture.drive,
                        None,
                        platform_version,
                    )
                    .expect("Q5 leaf-PCPS proof");
                black_box(proof)
            });
        },
    );

    // Bench 3: PCPS-carrier Q7 — `class IN [10 classes] AND
    // semester > floor` with `group_by=[class, semester]` and
    // `limit=10`. Uses the existing carrier executor.
    let q7_clauses = vec![
        WhereClause {
            field: "class".to_string(),
            operator: WhereOperator::In,
            value: Value::Array(all_classes.iter().map(|c| Value::Text(c.clone())).collect()),
        },
        WhereClause {
            field: "semester".to_string(),
            operator: WhereOperator::GreaterThan,
            value: semester_floor_value.clone(),
        },
    ];
    group.bench_function(
        "carrier_count_and_sum_per_class_semester_range_proof_limit_10",
        |b| {
            let q = DriveDocumentSumQuery {
                document_type,
                contract_id: fixture.data_contract.id().to_buffer(),
                document_type_name: DOCUMENT_TYPE_NAME.to_string(),
                index: by_class_semester_index,
                where_clauses: q7_clauses.clone(),
                sum_property: SUM_PROPERTY_NAME.to_string(),
            };
            b.iter(|| {
                let proof = q
                    .execute_carrier_aggregate_count_and_sum_with_proof(
                        &fixture.drive,
                        Some(10),
                        true,
                        None,
                        platform_version,
                    )
                    .expect("Q7 PCPS-carrier proof");
                black_box(proof)
            });
        },
    );

    // Bench 4: CountSumTree-carrier Q6 (no-proof analog) — `student IN
    // [10 students] AND semester == 20204`, grouped by `[student]`. Per
    // bucket is a point lookup (`semester == 20204`), so the
    // terminator is a CountSumTree (not PCPS). Built as a direct
    // path query for the bench since `DriveDocumentSumQuery` doesn't
    // have a "CountSumTree-carrier point-inner" helper.
    let students_10: Vec<[u8; 32]> = (0..10).map(student_id_bytes).collect();
    let bench_q6_floor = 20204_i64; // semester == 20204 (one cohort)
    group.bench_function(
        "group_by_student_in_no_proof_10_per_student_semester",
        |b| {
            b.iter_batched(
                || {
                    build_student_in_semester_eq_path_query(
                        fixture.data_contract.id().to_buffer(),
                        DOCUMENT_TYPE_NAME,
                        document_type,
                        &students_10,
                        bench_q6_floor,
                        platform_version,
                    )
                    .expect("Q6 path query")
                },
                |path_query| {
                    let proof = fixture
                        .drive
                        .grove
                        .get_proved_path_query(
                            &path_query,
                            None,
                            None,
                            &platform_version.drive.grove_version,
                        )
                        .value
                        .expect("Q6 student-In semester-EQ proof");
                    black_box(proof)
                },
                BatchSize::SmallInput,
            );
        },
    );

    group.finish();
}

// ---------------------------------------------------------------------
// Helpers: deterministic ID generators.
// ---------------------------------------------------------------------

/// Deterministic 32-byte student id derived from a small index.
/// First 8 bytes are big-endian u64 of `n`; remaining bytes are
/// the bitwise NOT of those 8 bytes, zero-padded — same pattern
/// as the sum bench's `recipient_id`, gives distinct ids that
/// sort monotonically by `n`.
fn student_id_bytes(n: u64) -> [u8; 32] {
    let mut id = [0u8; 32];
    id[..8].copy_from_slice(&n.to_be_bytes());
    for (i, byte) in n.to_be_bytes().iter().enumerate() {
        id[8 + i] = !byte;
    }
    id
}

/// One instructor per class — deterministic and stable. Distinct
/// from student ids (offset by `0x10` at byte 0) so probe output
/// can't accidentally collide a student and instructor id.
fn instructor_id_bytes(class_idx: u64) -> [u8; 32] {
    let mut id = [0u8; 32];
    id[0] = 0x10;
    id[1..9].copy_from_slice(&class_idx.to_be_bytes());
    id
}

fn class_name(class_idx: u64) -> String {
    CLASS_NAMES[class_idx as usize].to_string()
}

/// Deterministic document id from row index. Same shape pattern
/// as sum/count benches (BE row in high half, NOT row in next 8).
fn document_id(row: u64) -> [u8; 32] {
    let mut id = [0u8; 32];
    let document_number = row + 1;
    id[..8].copy_from_slice(&document_number.to_be_bytes());
    id[8..16].copy_from_slice(&(!document_number).to_be_bytes());
    id
}

// ---------------------------------------------------------------------
// Helpers: path-query builders for direct grovedb walks.
// ---------------------------------------------------------------------

/// Build a path query for `class IN [keys]` against the byClass
/// property-name tree. Each outer Key resolves to a CountSumTree
/// (since byClass declares both `countable` and `summable`). The
/// verifier-side `GroveDb::verify_query` returns one
/// `Element::CountSumTree(_, count, sum, _)` per resolved key.
fn build_class_in_path_query(
    contract_id: [u8; 32],
    document_type_name: &str,
    class_keys: &[Vec<u8>],
) -> PathQuery {
    let path = vec![
        vec![drive::drive::RootTree::DataContractDocuments as u8],
        contract_id.to_vec(),
        vec![1u8],
        document_type_name.as_bytes().to_vec(),
        b"class".to_vec(),
    ];
    let mut query = Query::new();
    for key in class_keys {
        query.insert_key(key.clone());
    }
    PathQuery::new(path, SizedQuery::new(query, None, None))
}

/// Build a path query for `student IN [keys] AND semester == X`
/// against the byStudentSemester compound index. The outer query
/// walks `student/<student_n>` (each a CountSumTree); the subquery
/// descends into the `semester` continuation and picks
/// `Key(serialize(semester == X))`. Inside the `semester` continuation
/// each cohort terminator is a CountSumTree (point, not PCPS, since
/// the where is `==` not a range). Verifier reads
/// `CountSumTree(_, count, sum, _)` per resolved (student, semester)
/// pair.
fn build_student_in_semester_eq_path_query(
    contract_id: [u8; 32],
    document_type_name: &str,
    document_type: dpp::data_contract::document_type::DocumentTypeRef,
    students: &[[u8; 32]],
    semester_value: i64,
    platform_version: &PlatformVersion,
) -> Result<PathQuery, drive::error::Error> {
    use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;

    let path = vec![
        vec![drive::drive::RootTree::DataContractDocuments as u8],
        contract_id.to_vec(),
        vec![1u8],
        document_type_name.as_bytes().to_vec(),
        b"student".to_vec(),
    ];
    let mut outer = Query::new();
    for s in students {
        outer.insert_key(s.to_vec());
    }
    // Subquery: descend into the `semester` continuation (one extra
    // path segment) and pick the serialized semester key.
    let serialized_semester = document_type.serialize_value_for_key(
        "semester",
        &Value::I64(semester_value),
        platform_version,
    )?;
    let mut subq = Query::new();
    subq.insert_key(serialized_semester);
    outer.set_subquery_path(vec![b"semester".to_vec()]);
    outer.set_subquery(subq);
    Ok(PathQuery::new(path, SizedQuery::new(outer, None, None)))
}

/// Median-of-N wall-clock timing for a closure that produces a
/// proof. One warmup discards the cold rocksdb-cache hit. Verbatim
/// copy of the sum bench's helper.
fn time_median<F: FnMut()>(iters: usize, mut f: F) -> std::time::Duration {
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

// ---------------------------------------------------------------------
// report_proof_sizes — 4 cases × proof bytes + median µs.
// ---------------------------------------------------------------------

/// Run each proof-emitting shape once and print byte size + median
/// µs. Same structure as sum bench's analog — Criterion measures
/// time for the load-bearing shapes; this is the lightweight
/// per-shape probe that publishes "Proof size: N bytes. Avg time: M
/// µs." numbers for the chapter.
fn report_proof_sizes(fixture: &AvgBenchFixture, platform_version: &PlatformVersion) {
    let document_type = fixture
        .data_contract
        .document_type_for_name(DOCUMENT_TYPE_NAME)
        .expect("grade doc type");
    let contract_id = fixture.data_contract.id().to_buffer();
    let all_classes: Vec<Vec<u8>> = (0..CLASS_COUNT)
        .map(|i| class_name(i).into_bytes())
        .collect();
    let students_10: Vec<[u8; 32]> = (0..10).map(student_id_bytes).collect();
    let semester_floor_value = Value::I64(fixture.range_floor as i64);

    // Case 1: class IN [10] → 10 CountSumTree branches via byClass.
    let case_1_pq = build_class_in_path_query(contract_id, DOCUMENT_TYPE_NAME, &all_classes);
    report_shape(
        fixture,
        platform_version,
        "group_by_class_in_proof_10_countsum_branches",
        || {
            fixture
                .drive
                .grove
                .get_proved_path_query(
                    &case_1_pq,
                    None,
                    None,
                    &platform_version.drive.grove_version,
                )
                .value
                .expect("class-In proof")
        },
    );

    // Case 2: Q5 leaf-PCPS (class==PHYS101 AND semester > floor).
    let by_class_semester_index = document_type
        .indexes()
        .get("byClassSemester")
        .expect("byClassSemester index");
    let q5 = DriveDocumentSumQuery {
        document_type,
        contract_id,
        document_type_name: DOCUMENT_TYPE_NAME.to_string(),
        index: by_class_semester_index,
        where_clauses: vec![
            WhereClause {
                field: "class".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("PHYS101".to_string()),
            },
            WhereClause {
                field: "semester".to_string(),
                operator: WhereOperator::GreaterThan,
                value: semester_floor_value.clone(),
            },
        ],
        sum_property: SUM_PROPERTY_NAME.to_string(),
    };
    report_shape(
        fixture,
        platform_version,
        "aggregate_count_and_sum_class_math_semester_range_proof",
        || {
            q5.execute_aggregate_count_and_sum_with_proof(&fixture.drive, None, platform_version)
                .expect("Q5 leaf-PCPS proof")
        },
    );

    // Case 3: Q7 PCPS-carrier (class IN [10] AND semester > floor).
    let q7 = DriveDocumentSumQuery {
        document_type,
        contract_id,
        document_type_name: DOCUMENT_TYPE_NAME.to_string(),
        index: by_class_semester_index,
        where_clauses: vec![
            WhereClause {
                field: "class".to_string(),
                operator: WhereOperator::In,
                value: Value::Array(
                    (0..CLASS_COUNT)
                        .map(|i| Value::Text(class_name(i)))
                        .collect(),
                ),
            },
            WhereClause {
                field: "semester".to_string(),
                operator: WhereOperator::GreaterThan,
                value: semester_floor_value.clone(),
            },
        ],
        sum_property: SUM_PROPERTY_NAME.to_string(),
    };
    report_shape(
        fixture,
        platform_version,
        "carrier_count_and_sum_per_class_semester_range_proof_limit_10",
        || {
            q7.execute_carrier_aggregate_count_and_sum_with_proof(
                &fixture.drive,
                Some(10),
                true,
                None,
                platform_version,
            )
            .expect("Q7 PCPS-carrier proof")
        },
    );

    // Case 4: Q6 CountSumTree-carrier (student IN [10] AND
    // semester==20204).
    let q6_pq = build_student_in_semester_eq_path_query(
        contract_id,
        DOCUMENT_TYPE_NAME,
        document_type,
        &students_10,
        20204,
        platform_version,
    )
    .expect("Q6 path query");
    report_shape(
        fixture,
        platform_version,
        "group_by_student_in_proof_10_per_student_semester_point",
        || {
            fixture
                .drive
                .grove
                .get_proved_path_query(&q6_pq, None, None, &platform_version.drive.grove_version)
                .value
                .expect("Q6 student-In semester-EQ proof")
        },
    );
}

/// Run one proof-emitting closure once, then 5× more to compute a
/// median wall-clock — same shape as sum bench's per-case probe.
fn report_shape<F>(
    fixture: &AvgBenchFixture,
    _platform_version: &PlatformVersion,
    label: &str,
    mut produce: F,
) where
    F: FnMut() -> Vec<u8>,
{
    let proof = produce();
    let bytes = proof.len();
    let median = time_median(5, || {
        let _ = produce();
    });
    // `rows=` reports the actually-inserted grade count (not the walked
    // triple count) so the proof-size header reflects the dataset's
    // real cardinality. See `AvgBenchFixture::inserted_count` doc.
    eprintln!(
        "[proof-size] rows={} {label}: {bytes} bytes  median={:.1} µs",
        fixture.inserted_count,
        median.as_secs_f64() * 1_000_000.0,
    );
}

// ---------------------------------------------------------------------
// report_group_by_matrix — Q1-Q7 outcomes (proof bytes / verified
// `(count, sum, avg)`) in a compact stderr-grep'able format.
// ---------------------------------------------------------------------

/// Walk Q1-Q7 and emit `[matrix] {label}: {outcome}` lines —
/// stderr-friendly, so a reviewer can grep `\[matrix\]` out of the
/// bench's output. Average bench keeps it light: every Q1-Q7
/// produces a verified `(count, sum, avg)` so the matrix just prints
/// the verified payload + proof size.
fn report_group_by_matrix(fixture: &AvgBenchFixture, platform_version: &PlatformVersion) {
    let document_type = fixture
        .data_contract
        .document_type_for_name(DOCUMENT_TYPE_NAME)
        .expect("grade doc type");
    let contract_id = fixture.data_contract.id().to_buffer();

    // Q1: unfiltered global average (primary-key CountSumTree).
    {
        let pq = primary_key_count_sum_path_query(contract_id, DOCUMENT_TYPE_NAME);
        emit_matrix_line(
            fixture,
            platform_version,
            "Q1 / where=(empty) / primary-key CountSumTree",
            &pq,
            VerifyShape::PointCountSum,
        );
    }
    // Q2: byClass — `class == "PHYS101"`.
    {
        let pq = build_key_path_query(
            contract_id,
            DOCUMENT_TYPE_NAME,
            "class",
            b"PHYS101".to_vec(),
        );
        emit_matrix_line(
            fixture,
            platform_version,
            "Q2 / where=class==\"PHYS101\" / byClass point",
            &pq,
            VerifyShape::PointCountSum,
        );
    }
    // Q3: byStudent — `student == student_050`.
    {
        let student_50 = student_id_bytes(50);
        let pq = build_key_path_query(
            contract_id,
            DOCUMENT_TYPE_NAME,
            "student",
            student_50.to_vec(),
        );
        emit_matrix_line(
            fixture,
            platform_version,
            "Q3 / where=student==student_050 / byStudent point",
            &pq,
            VerifyShape::PointCountSum,
        );
    }
    // Q4: byClassSemester point — `class==PHYS101 AND semester==20204`.
    {
        let pq = build_class_semester_point_path_query(
            contract_id,
            DOCUMENT_TYPE_NAME,
            document_type,
            "PHYS101",
            20204,
            platform_version,
        )
        .expect("Q4 path query");
        emit_matrix_line(
            fixture,
            platform_version,
            "Q4 / where=class==\"PHYS101\" AND semester==20204 / byClassSemester point",
            &pq,
            VerifyShape::PointCountSum,
        );
    }
    // Q5: byClassSemester AggregateCountAndSumOnRange.
    {
        let by_class_semester_index = document_type
            .indexes()
            .get("byClassSemester")
            .expect("byClassSemester index");
        let q5 = DriveDocumentSumQuery {
            document_type,
            contract_id,
            document_type_name: DOCUMENT_TYPE_NAME.to_string(),
            index: by_class_semester_index,
            where_clauses: vec![
                WhereClause {
                    field: "class".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Text("PHYS101".to_string()),
                },
                WhereClause {
                    field: "semester".to_string(),
                    operator: WhereOperator::GreaterThan,
                    value: Value::I64(fixture.range_floor as i64),
                },
            ],
            sum_property: SUM_PROPERTY_NAME.to_string(),
        };
        let pq = q5
            .aggregate_count_and_sum_path_query(platform_version)
            .expect("Q5 path query");
        match fixture
            .drive
            .grove
            .get_proved_path_query(&pq, None, None, &platform_version.drive.grove_version)
            .value
        {
            Ok(proof) => match GroveDb::verify_aggregate_count_and_sum_query(
                &proof,
                &pq,
                &platform_version.drive.grove_version,
            ) {
                Ok((_root_hash, count, sum)) => {
                    let avg = if count > 0 {
                        sum as f64 / count as f64
                    } else {
                        0.0
                    };
                    eprintln!(
                        "[matrix] Q5 / where=class==\"PHYS101\" AND semester > {} / AggregateCountAndSumOnRange\n         proof: {} bytes\n         verified: count={count} sum={sum} avg={avg:.4}",
                        fixture.range_floor,
                        proof.len(),
                    );
                }
                Err(e) => {
                    eprintln!("[matrix] Q5: verify_aggregate_count_and_sum_query error: {e:?}")
                }
            },
            Err(e) => eprintln!("[matrix] Q5: prover error: {e:?}"),
        }
    }
    // Q6: CountSumTree-carrier (student IN [10] AND semester==20204).
    {
        let students_10: Vec<[u8; 32]> = (0..10).map(student_id_bytes).collect();
        let pq = build_student_in_semester_eq_path_query(
            contract_id,
            DOCUMENT_TYPE_NAME,
            document_type,
            &students_10,
            20204,
            platform_version,
        )
        .expect("Q6 path query");
        emit_matrix_line(
            fixture,
            platform_version,
            "Q6 / where=student IN[10] AND semester==20204 / CountSumTree-carrier",
            &pq,
            VerifyShape::PerKeyCountSum,
        );
    }
    // Q7: PCPS-carrier (class IN [10] AND semester > floor, limit=10).
    {
        let by_class_semester_index = document_type
            .indexes()
            .get("byClassSemester")
            .expect("byClassSemester index");
        let q7 = DriveDocumentSumQuery {
            document_type,
            contract_id,
            document_type_name: DOCUMENT_TYPE_NAME.to_string(),
            index: by_class_semester_index,
            where_clauses: vec![
                WhereClause {
                    field: "class".to_string(),
                    operator: WhereOperator::In,
                    value: Value::Array(
                        (0..CLASS_COUNT)
                            .map(|i| Value::Text(class_name(i)))
                            .collect(),
                    ),
                },
                WhereClause {
                    field: "semester".to_string(),
                    operator: WhereOperator::GreaterThan,
                    value: Value::I64(fixture.range_floor as i64),
                },
            ],
            sum_property: SUM_PROPERTY_NAME.to_string(),
        };
        match q7.execute_carrier_aggregate_count_and_sum_with_proof(
            &fixture.drive,
            Some(10),
            true,
            None,
            platform_version,
        ) {
            Ok(proof) => match q7.verify_carrier_aggregate_count_and_sum_proof(
                &proof,
                Some(10),
                true,
                platform_version,
            ) {
                Ok((_root_hash, entries)) => {
                    eprintln!(
                        "[matrix] Q7 / where=class IN[10] AND semester > {} / PCPS-carrier\n         proof: {} bytes\n         verified: {} entries",
                        fixture.range_floor,
                        proof.len(),
                        entries.len(),
                    );
                    for (in_key, count, sum) in &entries {
                        let avg = if *count > 0 {
                            *sum as f64 / *count as f64
                        } else {
                            0.0
                        };
                        eprintln!(
                            "           in_key={} count={count} sum={sum} avg={avg:.4}",
                            display_segment_bytes(in_key),
                        );
                    }
                }
                Err(e) => eprintln!("[matrix] Q7: verify error: {e:?}"),
            },
            Err(e) => eprintln!("[matrix] Q7: prover error: {e:?}"),
        }
    }
}

/// Path query: `path/[prop]/[key]`. Used by Q2 and Q3 — each
/// resolves to one CountSumTree element.
fn build_key_path_query(
    contract_id: [u8; 32],
    document_type_name: &str,
    prop: &str,
    key: Vec<u8>,
) -> PathQuery {
    let path = vec![
        vec![drive::drive::RootTree::DataContractDocuments as u8],
        contract_id.to_vec(),
        vec![1u8],
        document_type_name.as_bytes().to_vec(),
        prop.as_bytes().to_vec(),
    ];
    let mut query = Query::new();
    query.insert_key(key);
    PathQuery::new(path, SizedQuery::new(query, None, None))
}

/// Primary-key path: `tip/[contract]/0x01/grade/[0x00]`. Resolves
/// to the doctype's CountSumTree (count=10 000, sum≈500 000 in
/// the bench fixture).
fn primary_key_count_sum_path_query(contract_id: [u8; 32], document_type_name: &str) -> PathQuery {
    let path = vec![
        vec![drive::drive::RootTree::DataContractDocuments as u8],
        contract_id.to_vec(),
        vec![1u8],
        document_type_name.as_bytes().to_vec(),
    ];
    let mut query = Query::new();
    query.insert_key(vec![0u8]);
    PathQuery::new(path, SizedQuery::new(query, None, None))
}

/// Build a Q4 path-query: `class/[class_key]/semester/[semester_key]`
/// — two property descents, terminator is a CountSumTree at the
/// `(class, semester)` cohort under byClassSemester.
fn build_class_semester_point_path_query(
    contract_id: [u8; 32],
    document_type_name: &str,
    document_type: dpp::data_contract::document_type::DocumentTypeRef,
    class: &str,
    semester: i64,
    platform_version: &PlatformVersion,
) -> Result<PathQuery, drive::error::Error> {
    use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;

    let serialized_semester = document_type.serialize_value_for_key(
        "semester",
        &Value::I64(semester),
        platform_version,
    )?;
    let path = vec![
        vec![drive::drive::RootTree::DataContractDocuments as u8],
        contract_id.to_vec(),
        vec![1u8],
        document_type_name.as_bytes().to_vec(),
        b"class".to_vec(),
        class.as_bytes().to_vec(),
        b"semester".to_vec(),
    ];
    let mut query = Query::new();
    query.insert_key(serialized_semester);
    Ok(PathQuery::new(path, SizedQuery::new(query, None, None)))
}

/// Which verify primitive applies to a given shape — drives the
/// `emit_matrix_line` branch that prints the verified payload.
enum VerifyShape {
    /// Single point lookup (Q1-Q4). `GroveDb::verify_query` returns
    /// one `Element::CountSumTree(_, count, sum, _)` per resolved key.
    PointCountSum,
    /// CountSumTree carrier (Q6). `GroveDb::verify_query` returns
    /// multiple `Element::CountSumTree` entries (one per outer Key).
    PerKeyCountSum,
}

fn emit_matrix_line(
    fixture: &AvgBenchFixture,
    platform_version: &PlatformVersion,
    label: &str,
    path_query: &PathQuery,
    shape: VerifyShape,
) {
    match fixture
        .drive
        .grove
        .get_proved_path_query(
            path_query,
            None,
            None,
            &platform_version.drive.grove_version,
        )
        .value
    {
        Ok(proof) => {
            match GroveDb::verify_query(&proof, path_query, &platform_version.drive.grove_version) {
                Ok((_root_hash, results)) => match shape {
                    VerifyShape::PointCountSum => {
                        let totals: Vec<(u64, i64)> = results
                            .iter()
                            .filter_map(|(_, _, elem)| element_count_and_sum(elem.as_ref()))
                            .collect();
                        let total_count: u64 = totals.iter().map(|(c, _)| c).sum();
                        let total_sum: i64 = totals.iter().map(|(_, s)| s).sum();
                        let avg = if total_count > 0 {
                            total_sum as f64 / total_count as f64
                        } else {
                            0.0
                        };
                        eprintln!(
                            "[matrix] {label}\n         proof: {} bytes\n         verified: count={total_count} sum={total_sum} avg={avg:.4} (over {} CountSumTree entries)",
                            proof.len(),
                            totals.len(),
                        );
                    }
                    VerifyShape::PerKeyCountSum => {
                        eprintln!(
                            "[matrix] {label}\n         proof: {} bytes\n         verified: {} CountSumTree entries",
                            proof.len(),
                            results.len(),
                        );
                        for (path, key, elem) in &results {
                            if let Some((count, sum)) = element_count_and_sum(elem.as_ref()) {
                                let avg = if count > 0 {
                                    sum as f64 / count as f64
                                } else {
                                    0.0
                                };
                                eprintln!(
                                    "           path-tail={} key={} count={count} sum={sum} avg={avg:.4}",
                                    display_segments(&path[(path.len().saturating_sub(2))..]),
                                    display_segment_bytes(key),
                                );
                            }
                        }
                    }
                },
                Err(e) => eprintln!("[matrix] {label}: verify_query error: {e:?}"),
            }
        }
        Err(e) => eprintln!("[matrix] {label}: prover error: {e:?}"),
    }
}

// ---------------------------------------------------------------------
// display_proofs — Q1-Q7 proof AST + verified payload (the
// load-bearing output for backfilling the chapter).
// ---------------------------------------------------------------------

/// Shape classifier for the verifier-side rebuild in `display_proofs`.
/// Each variant pins which `GroveDb::verify_*` call applies and what
/// the verified payload shape is.
enum DisplayShape {
    /// Primary-key, point, compound-point, or CountSumTree-carrier —
    /// `verify_query` returns one or more
    /// `Element::CountSumTree(_, count, sum, _)` entries.
    PointOrCountSumCarrier,
    /// Leaf-PCPS AggregateCountAndSumOnRange — verify via
    /// `verify_aggregate_count_and_sum_query` → `(root, count, sum)`.
    LeafPcps,
    /// PCPS-carrier — verify via
    /// `verify_aggregate_count_and_sum_query_per_key` →
    /// `(root, Vec<(in_key, count, sum)>)`. The limit / left_to_right
    /// the verifier uses are pinned by the matching
    /// [`PathQueryBuilder::PcpsCarrier`] variant — they live there
    /// because that's where the prover side also reads them, keeping
    /// prover/verifier byte-equality from drifting via two parallel
    /// copies of the same numbers.
    PcpsCarrier,
}

struct DisplayCase {
    label: &'static str,
    /// Either a pre-built path-query (for the cases the bench
    /// constructs directly) or a `DriveDocumentSumQuery` (for the
    /// cases that go through the `DriveDocumentSumQuery` helpers).
    /// The shape variant tells the runner which branch to take.
    path_query_builder: PathQueryBuilder,
    shape: DisplayShape,
}

/// Producer that yields the path query the prover walked. The two
/// `PcpsLeaf` / `PcpsCarrier` variants thread through
/// `DriveDocumentSumQuery::aggregate_count_and_sum_path_query` /
/// `…carrier_aggregate_count_and_sum_path_query`; everything else
/// is a hand-built `PathQuery`.
enum PathQueryBuilder {
    Direct(PathQuery),
    PcpsLeaf {
        clauses: Vec<WhereClause>,
    },
    PcpsCarrier {
        clauses: Vec<WhereClause>,
        limit: Option<u16>,
        left_to_right: bool,
    },
}

fn display_proofs(fixture: &AvgBenchFixture, platform_version: &PlatformVersion) {
    let document_type = fixture
        .data_contract
        .document_type_for_name(DOCUMENT_TYPE_NAME)
        .expect("grade doc type");
    let contract_id = fixture.data_contract.id().to_buffer();
    let by_class_semester_index = document_type
        .indexes()
        .get("byClassSemester")
        .expect("byClassSemester index");

    let all_classes_value: Vec<Value> = (0..CLASS_COUNT)
        .map(|i| Value::Text(class_name(i)))
        .collect();
    let students_10: Vec<[u8; 32]> = (0..10).map(student_id_bytes).collect();
    let semester_floor_value = Value::I64(fixture.range_floor as i64);

    let cases: Vec<DisplayCase> = vec![
        DisplayCase {
            label: "Q1 / where=(empty) / primary-key CountSumTree",
            path_query_builder: PathQueryBuilder::Direct(primary_key_count_sum_path_query(
                contract_id,
                DOCUMENT_TYPE_NAME,
            )),
            shape: DisplayShape::PointOrCountSumCarrier,
        },
        DisplayCase {
            label: "Q2 / where=class==\"PHYS101\" / byClass point",
            path_query_builder: PathQueryBuilder::Direct(build_key_path_query(
                contract_id,
                DOCUMENT_TYPE_NAME,
                "class",
                b"PHYS101".to_vec(),
            )),
            shape: DisplayShape::PointOrCountSumCarrier,
        },
        DisplayCase {
            label: "Q3 / where=student==student_050 / byStudent point",
            path_query_builder: PathQueryBuilder::Direct(build_key_path_query(
                contract_id,
                DOCUMENT_TYPE_NAME,
                "student",
                student_id_bytes(50).to_vec(),
            )),
            shape: DisplayShape::PointOrCountSumCarrier,
        },
        DisplayCase {
            label: "Q4 / where=class==\"PHYS101\" AND semester==20204 / byClassSemester point",
            path_query_builder: PathQueryBuilder::Direct(
                build_class_semester_point_path_query(
                    contract_id,
                    DOCUMENT_TYPE_NAME,
                    document_type,
                    "PHYS101",
                    20204,
                    platform_version,
                )
                .expect("Q4 path query"),
            ),
            shape: DisplayShape::PointOrCountSumCarrier,
        },
        DisplayCase {
            label:
                "Q5 / where=class==\"PHYS101\" AND semester > floor / AggregateCountAndSumOnRange",
            path_query_builder: PathQueryBuilder::PcpsLeaf {
                clauses: vec![
                    WhereClause {
                        field: "class".to_string(),
                        operator: WhereOperator::Equal,
                        value: Value::Text("PHYS101".to_string()),
                    },
                    WhereClause {
                        field: "semester".to_string(),
                        operator: WhereOperator::GreaterThan,
                        value: semester_floor_value.clone(),
                    },
                ],
            },
            shape: DisplayShape::LeafPcps,
        },
        DisplayCase {
            label: "Q6 / where=student IN[10] AND semester==20204 / CountSumTree-carrier",
            path_query_builder: PathQueryBuilder::Direct(
                build_student_in_semester_eq_path_query(
                    contract_id,
                    DOCUMENT_TYPE_NAME,
                    document_type,
                    &students_10,
                    20204,
                    platform_version,
                )
                .expect("Q6 path query"),
            ),
            shape: DisplayShape::PointOrCountSumCarrier,
        },
        DisplayCase {
            label: "Q7 / where=class IN[10] AND semester > floor / PCPS-carrier",
            path_query_builder: PathQueryBuilder::PcpsCarrier {
                clauses: vec![
                    WhereClause {
                        field: "class".to_string(),
                        operator: WhereOperator::In,
                        value: Value::Array(all_classes_value.clone()),
                    },
                    WhereClause {
                        field: "semester".to_string(),
                        operator: WhereOperator::GreaterThan,
                        value: semester_floor_value.clone(),
                    },
                ],
                limit: Some(10),
                left_to_right: true,
            },
            shape: DisplayShape::PcpsCarrier,
        },
    ];

    for case in cases {
        // Build the path query + produce proof bytes.
        let (path_query, proof_bytes_result) = match &case.path_query_builder {
            PathQueryBuilder::Direct(pq) => {
                let proof = fixture
                    .drive
                    .grove
                    .get_proved_path_query(pq, None, None, &platform_version.drive.grove_version)
                    .value
                    .map_err(|e| format!("{e:?}"));
                (pq.clone(), proof)
            }
            PathQueryBuilder::PcpsLeaf { clauses } => {
                let q = DriveDocumentSumQuery {
                    document_type,
                    contract_id,
                    document_type_name: DOCUMENT_TYPE_NAME.to_string(),
                    index: by_class_semester_index,
                    where_clauses: clauses.clone(),
                    sum_property: SUM_PROPERTY_NAME.to_string(),
                };
                let pq = q
                    .aggregate_count_and_sum_path_query(platform_version)
                    .expect("Q5 path query");
                let proof = q
                    .execute_aggregate_count_and_sum_with_proof(
                        &fixture.drive,
                        None,
                        platform_version,
                    )
                    .map_err(|e| format!("{e:?}"));
                (pq, proof)
            }
            PathQueryBuilder::PcpsCarrier {
                clauses,
                limit,
                left_to_right,
            } => {
                let q = DriveDocumentSumQuery {
                    document_type,
                    contract_id,
                    document_type_name: DOCUMENT_TYPE_NAME.to_string(),
                    index: by_class_semester_index,
                    where_clauses: clauses.clone(),
                    sum_property: SUM_PROPERTY_NAME.to_string(),
                };
                let pq = q
                    .carrier_aggregate_count_and_sum_path_query(
                        *limit,
                        *left_to_right,
                        platform_version,
                    )
                    .expect("Q7 path query");
                let proof = q
                    .execute_carrier_aggregate_count_and_sum_with_proof(
                        &fixture.drive,
                        *limit,
                        *left_to_right,
                        None,
                        platform_version,
                    )
                    .map_err(|e| format!("{e:?}"));
                (pq, proof)
            }
        };

        let proof_bytes = match proof_bytes_result {
            Ok(p) => p,
            Err(e) => {
                eprintln!(
                    "\n[display] {}\n  skipped — prover errored: {e}",
                    case.label
                );
                continue;
            }
        };

        // Median wall-clock for the Avg-time column.
        let median = match &case.path_query_builder {
            PathQueryBuilder::Direct(pq) => time_median(5, || {
                let _ = fixture
                    .drive
                    .grove
                    .get_proved_path_query(pq, None, None, &platform_version.drive.grove_version)
                    .value;
            }),
            PathQueryBuilder::PcpsLeaf { clauses } => time_median(5, || {
                let q = DriveDocumentSumQuery {
                    document_type,
                    contract_id,
                    document_type_name: DOCUMENT_TYPE_NAME.to_string(),
                    index: by_class_semester_index,
                    where_clauses: clauses.clone(),
                    sum_property: SUM_PROPERTY_NAME.to_string(),
                };
                let _ = q.execute_aggregate_count_and_sum_with_proof(
                    &fixture.drive,
                    None,
                    platform_version,
                );
            }),
            PathQueryBuilder::PcpsCarrier {
                clauses,
                limit,
                left_to_right,
            } => time_median(5, || {
                let q = DriveDocumentSumQuery {
                    document_type,
                    contract_id,
                    document_type_name: DOCUMENT_TYPE_NAME.to_string(),
                    index: by_class_semester_index,
                    where_clauses: clauses.clone(),
                    sum_property: SUM_PROPERTY_NAME.to_string(),
                };
                let _ = q.execute_carrier_aggregate_count_and_sum_with_proof(
                    &fixture.drive,
                    *limit,
                    *left_to_right,
                    None,
                    platform_version,
                );
            }),
        };

        eprintln!(
            "\n[display] {}\n  proof_size: {} bytes  median={:.1} µs\n  path: {}\n  items: {}",
            case.label,
            proof_bytes.len(),
            median.as_secs_f64() * 1_000_000.0,
            display_segments(&path_query.path),
            display_query_items(&path_query.query.query.items),
        );

        // Verify + print the verified payload, then the decoded
        // proof AST.
        match case.shape {
            DisplayShape::PointOrCountSumCarrier => {
                match GroveDb::verify_query(
                    &proof_bytes,
                    &path_query,
                    &platform_version.drive.grove_version,
                ) {
                    Ok((root_hash, results)) => {
                        eprintln!("  verified: root_hash={}", hex_bytes(&root_hash));
                        let mut sum_count = 0u64;
                        let mut sum_sum = 0i64;
                        for (path, key, elem) in &results {
                            let (count, sum) =
                                element_count_and_sum(elem.as_ref()).unwrap_or((0, 0));
                            let avg = if count > 0 {
                                sum as f64 / count as f64
                            } else {
                                0.0
                            };
                            eprintln!(
                                "    path={} key={} elem={} count={count} sum={sum} avg={avg:.4}",
                                display_segments(path),
                                display_segment_bytes(key),
                                display_element(elem.as_ref()),
                            );
                            sum_count = sum_count.saturating_add(count);
                            sum_sum = sum_sum.saturating_add(sum);
                        }
                        if results.len() > 1 {
                            let total_avg = if sum_count > 0 {
                                sum_sum as f64 / sum_count as f64
                            } else {
                                0.0
                            };
                            eprintln!(
                                "    aggregate: count={sum_count} sum={sum_sum} avg={total_avg:.4}"
                            );
                        }
                    }
                    Err(e) => eprintln!("  verify_query error: {e:?}"),
                }
            }
            DisplayShape::LeafPcps => {
                match GroveDb::verify_aggregate_count_and_sum_query(
                    &proof_bytes,
                    &path_query,
                    &platform_version.drive.grove_version,
                ) {
                    Ok((root_hash, count, sum)) => {
                        let avg = if count > 0 {
                            sum as f64 / count as f64
                        } else {
                            0.0
                        };
                        eprintln!(
                            "  verified: root_hash={} count={count} sum={sum} avg={avg:.4}",
                            hex_bytes(&root_hash),
                        );
                    }
                    Err(e) => eprintln!("  verify_aggregate_count_and_sum_query error: {e:?}"),
                }
            }
            DisplayShape::PcpsCarrier => {
                match GroveDb::verify_aggregate_count_and_sum_query_per_key(
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
                        for (in_key, count, sum) in &entries {
                            let avg = if *count > 0 {
                                *sum as f64 / *count as f64
                            } else {
                                0.0
                            };
                            eprintln!(
                                "    in_key={} count={count} sum={sum} avg={avg:.4}",
                                display_segment_bytes(in_key),
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("  verify_aggregate_count_and_sum_query_per_key error: {e:?}")
                    }
                }
            }
        }

        // Decoded proof AST — bincode big-endian, no length limit
        // (mirrors sum bench's config).
        let bincode_config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        match bincode::decode_from_slice::<GroveDBProof, _>(&proof_bytes, bincode_config) {
            Ok((decoded, _)) => eprintln!("  proof_ast:\n{decoded}"),
            Err(e) => eprintln!("  proof deserialize error: {e:?}"),
        }
    }
}

// ---------------------------------------------------------------------
// probe_value_tree_types — confirm every index's per-key value tree
// is the expected variant for the chapter's GroveDB layout diagram.
// ---------------------------------------------------------------------

/// Probe one representative entry per index and print the resolved
/// element variant + (count, sum). Confirms:
/// - byClass/PHYS101 → CountSumTree
/// - byStudent/<student_50> → CountSumTree
/// - bySemester/20205 → CountSumTree
/// - byClassSemester continuation at class=PHYS101 → PCPS-wrapped
fn probe_value_tree_types(fixture: &AvgBenchFixture, _platform_version: &PlatformVersion) {
    use drive::drive::RootTree;
    use grovedb_path::SubtreePath;

    let contract_id = fixture.data_contract.id().to_buffer();
    let grove_version = &PlatformVersion::latest().drive.grove_version;

    // serialize_value_for_key on `semester` returns the 8-byte BE
    // representation (per the integer-key encoding). 20205 in BE.
    let semester_20205 = (20205i64).to_be_bytes();
    // Convert to the unsigned representation used by serialize_value_for_key
    // — but we're probing the same path the prover walks, which lives
    // under `semester/<serialized>`; the actual bytes are produced by
    // serialize_value_for_key. Build via the same call here.
    let document_type = fixture
        .data_contract
        .document_type_for_name(DOCUMENT_TYPE_NAME)
        .expect("grade doc type");
    use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
    let serialized_semester = document_type
        .serialize_value_for_key("semester", &Value::I64(20205), PlatformVersion::latest())
        .expect("semester serialize");

    let cases: [(&'static str, &'static str, Vec<u8>); 3] = [
        ("byClass", "class", b"PHYS101".to_vec()),
        ("byStudent", "student", student_id_bytes(50).to_vec()),
        ("bySemester", "semester", serialized_semester.clone()),
    ];

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
            Ok(elem) => {
                let (count, sum) = element_count_and_sum(Some(&elem)).unwrap_or((0, 0));
                eprintln!(
                    "[probe] {label}: grade/{prop}/{} → {} {{ count={}, sum={} }}",
                    display_segment_bytes(&val),
                    element_variant_name(&elem),
                    count,
                    sum,
                );
            }
            Err(e) => eprintln!(
                "[probe] {label}: grade/{prop}/{} → grove.get error: {e:?}",
                display_segment_bytes(&val),
            ),
        }
    }

    // Probe the byClassSemester continuation: class/PHYS101/semester.
    {
        let parent_owned: Vec<Vec<u8>> = vec![
            vec![RootTree::DataContractDocuments as u8],
            contract_id.to_vec(),
            vec![1u8],
            DOCUMENT_TYPE_NAME.as_bytes().to_vec(),
            b"class".to_vec(),
            b"PHYS101".to_vec(),
        ];
        let parent: Vec<&[u8]> = parent_owned.iter().map(|v| v.as_slice()).collect();
        match fixture
            .drive
            .grove
            .get(SubtreePath::from(parent.as_slice()), b"semester", None, grove_version)
            .unwrap()
        {
            Ok(elem) => {
                let (count, sum) = element_count_and_sum(Some(&elem)).unwrap_or((0, 0));
                eprintln!(
                    "[probe] byClassSemester /semester continuation under class=PHYS101 → {} {{ count={}, sum={} }}",
                    element_variant_name(&elem),
                    count,
                    sum,
                );
            }
            Err(e) => eprintln!(
                "[probe] byClassSemester /semester continuation under class=PHYS101 → grove.get error: {e:?}",
            ),
        }
        // And probe one bucket inside: class/PHYS101/semester/<20205>.
        let inner_parent_owned: Vec<Vec<u8>> = vec![
            vec![RootTree::DataContractDocuments as u8],
            contract_id.to_vec(),
            vec![1u8],
            DOCUMENT_TYPE_NAME.as_bytes().to_vec(),
            b"class".to_vec(),
            b"PHYS101".to_vec(),
            b"semester".to_vec(),
        ];
        let inner_parent: Vec<&[u8]> = inner_parent_owned.iter().map(|v| v.as_slice()).collect();
        match fixture
            .drive
            .grove
            .get(
                SubtreePath::from(inner_parent.as_slice()),
                &serialized_semester,
                None,
                grove_version,
            )
            .unwrap()
        {
            Ok(elem) => {
                let (count, sum) = element_count_and_sum(Some(&elem)).unwrap_or((0, 0));
                eprintln!(
                    "[probe] byClassSemester /class/PHYS101/semester/20205 cohort → {} {{ count={}, sum={} }}",
                    element_variant_name(&elem),
                    count,
                    sum,
                );
            }
            Err(e) => eprintln!(
                "[probe] byClassSemester /class/PHYS101/semester/20205 → grove.get error: {e:?}",
            ),
        }
    }

    // Also probe semester=20205. Bytes were computed from a Value::I64
    // via the same serializer above. Print for reference.
    let _ = semester_20205; // (Kept for readers cross-referencing; not used directly.)
}

// ---------------------------------------------------------------------
// Helpers: display + element introspection.
// ---------------------------------------------------------------------

/// Read `(count, sum)` off a `CountSumTree` /
/// `ProvableCountSumTree` / `ProvableCountProvableSumTree` element.
/// Returns `None` for variants without both axes — surfacing a
/// missing-pair as `None` rather than `(0, 0)` so the matrix output
/// flags wrong-shape probes for human review.
fn element_count_and_sum(elem: Option<&grovedb::Element>) -> Option<(u64, i64)> {
    use grovedb::Element;
    match elem? {
        Element::CountSumTree(_, count, sum, _) => Some((*count, *sum)),
        Element::ProvableCountSumTree(_, count, sum, _) => Some((*count, *sum)),
        Element::ProvableCountProvableSumTree(_, count, sum, _) => Some((*count, *sum)),
        _ => None,
    }
}

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
        Element::ProvableCountProvableSumTree(_, _, _, _) => "ProvableCountProvableSumTree",
        Element::Tree(_, _) => "Tree (NormalTree)",
        Element::Item(_, _) => "Item",
        Element::SumItem(_, _) => "SumItem",
        Element::ItemWithSumItem(_, _, _) => "ItemWithSumItem",
        Element::Reference(_, _, _) => "Reference",
        Element::ReferenceWithSumItem(_, _, _, _) => "ReferenceWithSumItem",
        _ => "(other-variant)",
    }
}

fn display_element(elem: Option<&grovedb::Element>) -> String {
    match elem {
        None => "(absent)".to_string(),
        Some(e) => {
            let (count, sum) = element_count_and_sum(Some(e)).unwrap_or((0, 0));
            format!("{} {{ count={count}, sum={sum} }}", element_variant_name(e))
        }
    }
}

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

fn display_query_items(items: &[QueryItem]) -> String {
    let parts: Vec<String> = items
        .iter()
        .map(|item| match item {
            QueryItem::Key(k) => format!("Key({})", display_segment_bytes(k)),
            QueryItem::Range(r) => format!(
                "Range({}..{})",
                display_segment_bytes(&r.start),
                display_segment_bytes(&r.end)
            ),
            QueryItem::RangeInclusive(r) => format!(
                "RangeInclusive({}..={})",
                display_segment_bytes(r.start()),
                display_segment_bytes(r.end())
            ),
            QueryItem::RangeFull(_) => "RangeFull".to_string(),
            QueryItem::RangeFrom(r) => {
                format!("RangeFrom({}..)", display_segment_bytes(&r.start))
            }
            QueryItem::RangeTo(r) => {
                format!("RangeTo(..{})", display_segment_bytes(&r.end))
            }
            QueryItem::RangeToInclusive(r) => {
                format!("RangeToInclusive(..={})", display_segment_bytes(&r.end))
            }
            QueryItem::RangeAfter(r) => {
                format!("RangeAfter({}..)", display_segment_bytes(&r.start))
            }
            QueryItem::RangeAfterTo(r) => format!(
                "RangeAfterTo({}..{})",
                display_segment_bytes(&r.start),
                display_segment_bytes(&r.end)
            ),
            QueryItem::RangeAfterToInclusive(r) => format!(
                "RangeAfterToInclusive({}..={})",
                display_segment_bytes(r.start()),
                display_segment_bytes(r.end())
            ),
            other => format!("{:?}", other),
        })
        .collect();
    format!("[{}]", parts.join(", "))
}

fn display_segment_bytes(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) if s.chars().all(|c| !c.is_control()) => format!("{:?}", s),
        _ => format!("0x{}", hex_bytes(bytes)),
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// ---------------------------------------------------------------------
// Environment + fixture-path helpers.
// ---------------------------------------------------------------------

fn row_count() -> u64 {
    env_u64("DASH_PLATFORM_AVG_BENCH_ROWS").unwrap_or(DEFAULT_ROW_COUNT)
}

fn batch_size() -> u64 {
    env_u64("DASH_PLATFORM_AVG_BENCH_BATCH_SIZE").unwrap_or(DEFAULT_BATCH_SIZE)
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
    if let Ok(path) = env::var("DASH_PLATFORM_AVG_BENCH_DB") {
        return PathBuf::from(path);
    }
    env::temp_dir().join(format!(
        "dash-platform-document-average-bench-v{FIXTURE_SCHEMA_VERSION}-rows-{row_count}"
    ))
}

fn fixture_marker(row_count: u64) -> String {
    let protocol_version = PlatformVersion::latest().protocol_version;
    format!(
        "schema_version={FIXTURE_SCHEMA_VERSION}\nprotocol_version={protocol_version}\nrows={row_count}\nstudents={STUDENT_COUNT}\nclasses={CLASS_COUNT}\nsemesters={SEMESTER_COUNT}\n"
    )
}

criterion_group!(average_query_worst_cases, document_average_worst_case);
criterion_main!(average_query_worst_cases);
