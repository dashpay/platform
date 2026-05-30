# Shielded-pool genesis snapshot — design

Pivots PR #3732 away from runtime seeding into a **shielded-pool-specific
snapshot bake + RocksDB ingest** model. One-shot tool, no future-proof
genericity — every Element kind that ever needs this gets its own dedicated
path (YAGNI until a second case appears).

## 1. Problem statement

The Option-A path that the current PR implements (run the seeder at
`InitChain`) takes **~3h 41m for 500_000 notes on macOS Docker** — release
profile, single attempt, no retries. CPU work is ~100 s; the remaining
~3h 40m is GroveDB write amplification through Docker Desktop's file-share
layer. Unusable for any reset-driven dev loop.

The Sinsemilla math is fundamentally sequential per-tree (each append
depends on prior frontier state), so per-insert hot-path optimisations cap
at single-digit minutes. The only way to drop the cost meaningfully is to
**avoid doing the 500k writes at runtime**.

## 2. Approach

Move the writes to an **offline bake** that runs inside docker buildkit's
linux VM (fast native fsync), serialise the resulting subtree's RocksDB
state into a portable snapshot file, ship that file in the image, and have
`InitChain` `ingest_external_file_cf` the SST contents back into the live DB
at boot. After ingest, **replay the cmx stream to recompute the two roots
(`bulk_state_root` + `sinsemilla_root`) and cross-validate** against the
header's values — any tampering or version skew surfaces here, before the
parent Merk is touched. Then `Element::insert_subtree` writes the parent
Merk leaf and Merk propagation runs normally.

Three components:

```
┌──────────────────────────────────────┐   ┌───────────────────────────┐
│  snapshot-bake CLI (one-shot)        │   │  InitChain (every reset)  │
│  rs-drive-abci src/bin/              │   │  rs-drive-abci genesis    │
│  snapshot-bake.rs                    │   │  path                     │
│                                      │   │                           │
│  1. open tmp grovedb in /tmp         │   │  1. create_genesis_state  │
│  2. seed_shielded_pool_with_config() │   │     builds parent tree    │
│  3. dump_shielded_subtree(...)       │   │  2. apply_shielded_       │
│                                      │   │     snapshot_if_set(...)  │
│                                      │   │     (inside the same txn) │
└──────────────────────────────────────┘   └───────────────────────────┘
                  │                                       ▲
                  ▼                                       │
            ┌──────────────────────────────────────────────────┐
            │  rs-drive-abci::shielded_snapshot module         │
            │  - dump_shielded_subtree(grove, w) -> Stats      │
            │  - apply_shielded_snapshot(grove, r, txn) -> ()  │
            │  - shielded-specific. No dispatch table.         │
            └──────────────────────────────────────────────────┘
```

Both `dump` and `apply` live in `rs-drive-abci` (not `rs-drive`) because they
co-locate with the seeder and the existing shielded subtree code. No new
public surface in `rs-drive`. The grovedb-storage surface additions (per-CF
iteration, raw-db access) live in `grovedb-storage` itself because they're
foundational.

## 3. Snapshot file format

Single self-describing binary file, shielded-specific. The two crypto roots
are **carried in the header AND independently recomputed at apply time** —
the header values are a hint the apply path cross-validates against, not a
source of truth.

```
+──────────────────+──────+─────────────────────────────────────────────+
| Field            | Size | Description                                  |
+──────────────────+──────+─────────────────────────────────────────────+
| magic            | 8 B  | "DRVSHLD\0"                                  |
| format_version   | u32  | 1 (this revision). Bump = breaking change.   |
| grovedb_git_sha  | 20 B | git SHA of grovedb the bake ran against. Hard|
|                  |      | error if mismatched with the runtime build.  |
| total_count      | u64  | number of cmx commitments in the subtree     |
| chunk_power      | u8   | BulkAppendTree chunk_power (≤ 16 — enforced  |
|                  |      | at apply time as DoS sanity)                 |
| flags            | u8   | Element::CommitmentTree flags byte           |
| sinsemilla_root  | 32 B | Pallas-base — header hint, cross-validated   |
|                  |      | at apply time by replaying the cmx stream.   |
| bulk_state_root  | 32 B | blake3("bulk_state" || mmr_root || buffer    |
|                  |      | hash) — header hint, cross-validated.        |
| default_sst_len  | u64  | length of default-CF SST blob                |
| default_sst      | var  | SST containing META_KEY, chunk blobs, MMR    |
|                  |      | nodes — produced by SstFileWriter            |
| aux_sst_len      | u64  | length of aux-CF SST blob                    |
| aux_sst          | var  | SST containing the Sinsemilla frontier       |
|                  |      | (`__ct_data__` key)                          |
| checksum         | 32 B | blake3 over everything above                 |
+──────────────────+──────+─────────────────────────────────────────────+
```

**CF routing is fixed and explicit.** Per feasibility review:
`DataBulkStore` (used by `commitment_tree_insert` line 249) calls
`ctx.put/get` which route to **default CF**. So META_KEY (`b"M"`), chunk
blobs (`e{u64}`), tail buffer (`b{u32}`), and chunk-MMR nodes (`m{u64}`)
**all live in default CF**. The Sinsemilla frontier
(`COMMITMENT_TREE_DATA_KEY = b"__ct_data__"`) is the only thing in **aux**
(written via `put_aux` at `commitment_tree.rs:288-291`). Dump captures both;
apply ingests both.

**Why embedded SST bytes vs (k,v) tuples:** lets the bake emit final
RocksDB-ready bytes (no per-record fsync), and the apply side writes the
section to a tmp file and calls `ingest_external_file_cf` once per CF.

## 4. Module API

In `packages/rs-drive-abci/src/shielded_snapshot/mod.rs`:

```rust
pub fn dump_shielded_subtree(
    grove: &GroveDb,
    transaction: TransactionArg,
    out_path: &Path,
    platform_version: &PlatformVersion,
) -> Result<DumpStats, ShieldedSnapshotError>;

pub fn apply_shielded_snapshot(
    grove: &GroveDb,
    transaction: TransactionArg,
    snapshot_path: &Path,
    platform_version: &PlatformVersion,
) -> Result<ApplyStats, ShieldedSnapshotError>;
```

Errors are structured. `ShieldedSnapshotError::PartiallyApplied { ingested_cfs, failed_cf, cause }` lets the caller distinguish "no-op,
retry" from "DB in partial state, wipe and re-bootstrap."

**`dump_shielded_subtree`** is shielded-specific. The subtree path is
hardcoded as the well-known shielded pool location. The dump iterates the
subtree's prefix in both default and aux CFs, emitting two `SstFileWriter`
streams. Reads the parent-Merk leaf to capture `total_count`, `chunk_power`,
`flags`, `sinsemilla_root`, `bulk_state_root` for the header.

**`apply_shielded_snapshot`** does, in order:

1. Read + validate header (magic, `format_version == 1`, `grovedb_git_sha ==
   runtime git SHA`, `chunk_power ≤ 16`, checksum).
2. Extract each section's SST bytes to a tmp file.
3. Call `db.ingest_external_file_cf` on default CF, then aux CF, with
   `IngestExternalFileOptions { allow_global_seqno: false, write_global_seqno: false, ..Default::default() }`. (Explicit per crypto review
   D2 — global seqno injection would let a malicious snapshot poison
   sequence ordering.)
4. Reconstruct `BulkAppendTree::load_from_store(store, total_count, chunk_power)`. Reads META_KEY etc from default CF.
5. Read the Sinsemilla frontier from aux CF, deserialize.
6. **Cross-validate** (per crypto F1/D1):
   - Recompute `bulk_state_root` from the loaded BulkAppendTree's
     `mmr_root` + `buffer_hash`. Compare to header. Mismatch → fatal.
   - Walk the frontier's recorded leaves (or replay cmx → fresh frontier),
     compare resulting `sinsemilla_root` to header. Mismatch → fatal.
7. Build `Element::CommitmentTree(sinsemilla_root, total_count, chunk_power, flags)` and call `element.insert_subtree(&mut parent_merk, key, bulk_state_root, txn)`.
8. `propagate_changes_with_transaction(txn)`.

Step 6 is the non-negotiable fix from crypto review — without it, a
corrupted snapshot could ingest a poisoned root that the chain then blesses.

## 5. Bake tool

`packages/rs-drive-abci/src/bin/snapshot-bake.rs`. CLI:

```
snapshot-bake shielded-pool --out <path>
```

No dispatch table, no subcommand registry. Single hardcoded operation: open
tmp grovedb, run the existing
`seed_shielded_pool_with_config(&ShieldedSeedConfig::sdk_test_data())`, call
`dump_shielded_subtree(grove, &mut file)`. Per scope review.

The bake binary is built with the **same cfg flags as the runtime
drive-abci** (specifically `create_sdk_test_data`). Per crypto review D5 —
mismatched cfg would let the bake produce a shape the runtime can't accept.
Dockerfile enforces this by deriving both from the same builder stage.

`DEFAULT_OPTS` (RocksDB Options) is shared between bake and apply paths via
a `pub fn snapshot_db_options() -> Options` in `grovedb-storage` so the SST
files are guaranteed compatible.

## 6. InitChain integration

In `packages/rs-drive-abci/src/execution/platform_events/initialization/create_genesis_state/mod.rs`,
**inside the existing InitChain transaction**, after the parent tree
structure (`/platform/`, `/platform/shielded_pool/`, ...) is built but
before `create_data_for_shielded_pool` would have run:

```rust
if let Ok(snapshot_path) = std::env::var("DRIVE_SHIELDED_SNAPSHOT") {
    let mut file = File::open(&snapshot_path)
        .map_err(|e| Error::Execution(format!(
            "DRIVE_SHIELDED_SNAPSHOT={snapshot_path} unreadable: {e}"
        ).into()))?;
    shielded_snapshot::apply_shielded_snapshot(
        &self.drive.grove,
        &mut file,
        transaction,
    ).map_err(Error::Execution)?;
}
```

Transactional contract (per feasibility F3 + your direction):

- The ingest_external_file_cf calls **bypass the transaction layer** — they
  write SSTs directly to default+aux CFs at the DB level.
- The parent-Merk leaf write (`element.insert_subtree`) **goes through the
  transaction** like any other Merk op.
- Failure mode: if the InitChain transaction aborts AFTER ingest but BEFORE
  commit, the default+aux data persists with no parent leaf pointing at it.
  This is an orphan, but **InitChain abort already requires wipe-and-restart
  recovery** (a half-bootstrapped chain isn't usable), so the orphan
  resolves naturally.
- The cmx-replay cross-validation in step 6 of §4 happens **before** the
  parent leaf write. If validation fails, the transaction proceeds with no
  parent leaf and the InitChain handler returns an error, triggering
  exactly the wipe-and-restart path the abort case relies on.

`create_data_for_shielded_pool` branches on the `DRIVE_SHIELDED_SNAPSHOT`
env var:

- **Snapshot fast-path (env var set):** the snapshot file is applied via SST
  ingest. Any failure here — file missing, magic mismatch, format/version
  skew, checksum failure, or `combined_root` cross-validation drift — is
  **fatal**: `apply_shielded_snapshot` returns a `ShieldedSnapshotError` and
  InitChain surfaces it as `Error::Execution(...)` and aborts. There is no
  silent fallback to the seeder once the env var is set; the operator is
  expected to fix the snapshot rather than get different state than asked for.
- **Runtime seeding fallback (env var unset):** the chain seeds the shielded
  pool at genesis from `ShieldedSeedConfig::sdk_test_data()` (the slow
  Sinsemilla runtime path). This is the path used when the binary is built
  with `create_sdk_test_data` but no baked snapshot is shipped.

The Dockerfile only exports `DRIVE_SHIELDED_SNAPSHOT` when the bake stage
actually produced a snapshot file (`SDK_TEST_DATA=true`); otherwise the bake
stage leaves a `.no-shielded-snapshot` sentinel and the env var is left unset
so the runtime seeder fallback runs. The bake-then-apply route is the fast,
recommended way to populate the shielded pool at genesis.

`record_shielded_pool_anchor_if_changed(height=1)` runs after the snapshot
apply, same as before, so the anchor matches the snapshot's frontier.

## 7. Dockerfile + dashmate plumbing

Bake stage runs in the linux VM where fsync is fast:

```dockerfile
# After build-drive-abci, gated on SDK_TEST_DATA build arg:
FROM build-drive-abci AS bake-shielded-snapshot
ARG SDK_TEST_DATA=false
RUN if [ "$SDK_TEST_DATA" = "true" ]; then \
        cargo build --release --bin snapshot-bake \
            --features create_sdk_test_data && \
        target/release/snapshot-bake shielded-pool \
            --out /artifacts/shielded-pool.snap ; \
    else \
        mkdir -p /artifacts && \
        : > /artifacts/shielded-pool.snap.absent ; \
    fi

FROM <runtime-base> AS drive-abci
COPY --from=build-drive-abci /artifacts/drive-abci /usr/bin/drive-abci
COPY --from=bake-shielded-snapshot /artifacts/ /opt/dashmate/snapshots/
```

`SDK_TEST_DATA=true` is the existing dashmate build arg
(`platform.drive.abci.docker.build.buildArgs.SDK_TEST_DATA`). Per crypto D5,
the bake binary is compiled with `--features create_sdk_test_data` so it
matches the runtime's cfg view of `Element`/seeder shape.

Per-platform bake under `buildx` for cross-arch (arm64 mac vs amd64
production). SST files embed compression block decisions made by
`SstFileWriter` — these must match the apply-side `Options`, which is why
both sides pull from `grovedb_storage::snapshot_db_options()`.

Dashmate side: drive-abci container env adds

```yaml
environment:
  - DRIVE_SHIELDED_SNAPSHOT=${DRIVE_SHIELDED_SNAPSHOT:-/opt/dashmate/snapshots/shielded-pool.snap}
```

Default points at the in-image snapshot. Empty string or unset = no apply
(genesis runs without shielded data). Operators wanting a custom snapshot
mount-bind a different file and set the env.

**Removed in this PR:**
- `CARGO_BUILD_PROFILE=release` workaround in `scripts/setup_local_network.sh` (no longer needed — release profile speedup
  was for the runtime seeder, which is gone).
- `create_data_for_shielded_pool` runtime path.
- Slow-seeder fallback option. Hard requirement on the snapshot file when
  `create_sdk_test_data` is built in.

## 8. Correctness reasoning (Phase 1 only — see §15 for Phase 2 deltas)

> **Scope:** this section's "byte-identical to what the runtime would
> have written" claim holds for the Phase 1 design (full-Sinsemilla
> seed in the bake). Phase 2 (§15) deliberately drops that property for
> the speedup. After Phase 2 lands, the bake produces a snapshot whose
> `combined_root` is self-consistent but **not equal** to what a
> runtime-seeded chain would produce — see §15.5.

**Why bulk-loaded data + cross-validation + Merk propagation produces the
same root as runtime seeding:**

1. The bake tool runs the **exact same `commitment_tree_insert_op` sequence**
   the runtime would. The resulting key-value pairs in storage are
   byte-identical to what the runtime would have written. Same RNG seed
   (`ShieldedSeedConfig::sdk_test_data().rng_seed = 0xDEAD_BEEF`), same note
   generator path, same Sinsemilla state machine.

2. `dump_shielded_subtree` enumerates those key-value pairs from the tmp DB
   (default CF: META_KEY + chunk blobs + buffer + MMR nodes; aux CF:
   Sinsemilla frontier) and writes them into SST files. RocksDB SST format
   preserves key+value bytes byte-for-byte.

3. `apply_shielded_snapshot` ingests both SSTs. RocksDB's bulk-load path
   writes to the LSM tree without going through WAL, but post-ingest the
   key-value visibility is identical to having done individual `put` calls.
   `IngestExternalFileOptions { allow_global_seqno: false }` prevents
   sequence-number injection that could let a poisoned snapshot reorder
   later writes.

4. **Cross-validation (step 6 of §4) is what makes this safe.** We don't
   trust the header's `bulk_state_root` and `sinsemilla_root`. We reload
   the BulkAppendTree from the freshly-ingested storage, recompute
   `bulk_state_root = blake3("bulk_state" || mmr_root || buffer_hash)`, and
   replay the cmx stream through a fresh Sinsemilla frontier to recompute
   `sinsemilla_root`. Both must equal the header values byte-for-byte. If
   anyone tampered with the SST data, or if grovedb's storage layout
   changed between bake and apply, the recomputed roots diverge and we
   fail before touching the parent Merk.

5. `element.insert_subtree(parent, key, bulk_state_root, txn)` writes the
   parent Merk leaf with the validated `bulk_state_root` as the child hash,
   inside the InitChain transaction.

6. `propagate_changes_with_transaction(txn)` runs the same hash propagation
   the runtime would. After commit, every ancestor Merk root matches what
   the slow seeder would have produced.

7. A regression test (§10 #6) bakes the snapshot, applies to chain A, runs
   the runtime seeder on chain B against a tmp grovedb, asserts
   `grove_a.root_hash() == grove_b.root_hash()` byte-for-byte.

**Threat model (per crypto D4):**

- **In scope:** corrupted/truncated SST bytes (cross-validation + RocksDB
  ingest validation catches), header tampering with roots (cross-validation
  catches), grovedb version skew (git SHA hard error).
- **Out of scope by design:** cross-devnet snapshot replay (devnets share
  the same shielded pool layout — that's the point), image-supply-chain
  trust (we trust the docker image's bake stage produced honest bytes; this
  is the same trust boundary as the rest of the image).
- **Not addressed:** snapshot signing. Devnet-only; signing adds key-mgmt
  surface for no current threat. Decision deferred. If we ever ship this
  for testnet/mainnet, revisit.

## 9. Edge cases & error handling

| Case | Behaviour |
|---|---|
| `DRIVE_SHIELDED_SNAPSHOT` unset, `create_sdk_test_data` NOT built | No-op. Chain boots normally with empty shielded pool. |
| `DRIVE_SHIELDED_SNAPSHOT` unset, `create_sdk_test_data` built | **Fatal**: `Error::Execution("DRIVE_SHIELDED_SNAPSHOT required when built with create_sdk_test_data")`. Forces the bake stage to be wired correctly. |
| File doesn't exist | Fatal: `Error::Execution(SnapshotFileNotFound { path })` |
| Magic mismatch | Fatal: `ShieldedSnapshotError::InvalidMagic` |
| `format_version` mismatch | Fatal: `ShieldedSnapshotError::FormatVersionMismatch { expected, found }` |
| `grovedb_git_sha` mismatch | Fatal: `ShieldedSnapshotError::GrovedbRevMismatch { expected, found }` |
| `chunk_power > 16` | Fatal: `ShieldedSnapshotError::ChunkPowerTooLarge { got, max: 16 }` |
| Checksum mismatch | Fatal: `ShieldedSnapshotError::Corrupted` |
| RocksDB ingest fails (CF overlap, etc.) | Fatal: `ShieldedSnapshotError::IngestFailed { cf, cause }` |
| `BulkAppendTree::load_from_store` fails | Fatal: `ShieldedSnapshotError::CorruptedAfterIngest { cause }` — at this point the default CF data is on disk; caller should treat as PartiallyApplied. |
| `bulk_state_root` cross-validation mismatch | Fatal: `ShieldedSnapshotError::BulkStateRootMismatch { expected, computed }` |
| `sinsemilla_root` cross-validation mismatch | Fatal: `ShieldedSnapshotError::SinsemillaRootMismatch { expected, computed }` |
| Parent subtree (`/platform/shielded_pool/`) missing in target DB | Fatal: `ShieldedSnapshotError::ParentSubtreeMissing` — snapshot must run after `create_genesis_state` builds the parent tree. |

## 10. Testing strategy

Tests live in `packages/rs-drive-abci/tests/shielded_snapshot/`:

1. **Roundtrip** — small (N=16) shielded subtree:
   - Build via normal grovedb ops (existing `seed_shielded_pool_with_config`)
   - `dump_shielded_subtree` to a buffer
   - `apply_shielded_snapshot` into a fresh DB (with parent tree pre-built)
   - Assert grovedb root hash matches between source and target
   - Assert recorded anchor matches
2. **Same-host determinism** (per crypto D6) — bake the shielded subtree
   twice on the same host with the same `RUSTC_BOOTSTRAP`/cfg flags; assert
   snapshot file bytes are identical. Cross-host determinism is **not**
   asserted (relies on RocksDB internals).
3. **Format-version refusal** — craft a snapshot with `format_version=2`;
   assert `FormatVersionMismatch`.
4. **Git-SHA refusal** — craft a snapshot with a different `grovedb_git_sha`;
   assert `GrovedbRevMismatch`.
5. **Corruption** — flip bits in default-CF SST, in aux-CF SST, in header
   roots; assert `Corrupted` (checksum) or root-mismatch (cross-validation)
   for each.
6. **Equivalence with runtime seeder** (RETIRED under Phase 2) — bake
   snapshot, apply to chain A; run runtime seeder on chain B; assert
   `grove_a.root_hash() == grove_b.root_hash()`. This test was valid
   under Phase 1 (full-Sinsemilla bake) but **fails under Phase 2**
   because the bake uses `append_many_without_frontier` for filler,
   producing a different `combined_root` than a runtime-seeded chain.
   Remove or `#[cfg]`-split when Phase 2 lands; replace with a
   "bake-side determinism" test (same config → same snapshot bytes
   across two bakes).
7. **Cross-validation catches header tampering** — bake snapshot, flip the
   `bulk_state_root` field in the header, fix the outer checksum; assert
   `BulkStateRootMismatch`. Same with `sinsemilla_root` →
   `SinsemillaRootMismatch`.
8. **Fuzz** (per crypto D7) — `cargo-fuzz` target on `read_header`. Verify
   no panics on arbitrary byte input.
9. **chunk_power bound** — craft snapshot with `chunk_power=17`; assert
   `ChunkPowerTooLarge`.
10. **End-to-end (against snapshot-bake CLI)**:
    - Bake the shielded subtree to a tmp file
    - Open a fresh `TempPlatform`, run `create_genesis_state` with
      `DRIVE_SHIELDED_SNAPSHOT` pointing at the tmp file
    - Assert `shielded_pool_notes_count == 500_000`
    - Assert recorded anchor at height 1 matches the snapshot's frontier
11. **Functional wallet sync** — existing
    `wallet_a_recovers_deterministic_balance_via_manager_sync` works
    unchanged against a snapshot-loaded chain.

## 11. Open questions & risks

| # | Risk | Mitigation |
|---|---|---|
| 1 | SST cross-arch portability (arm64 ↔ amd64). | Bake per-`--platform` under buildx. Image layers already platform-tagged. |
| 2 | grovedb may cache subtree metadata in memory; raw ingest could leave caches stale. | `apply_shielded_snapshot` is the FIRST grovedb op that touches the shielded subtree path in the InitChain txn. No reads happen before it. |
| 3 | `ingest_external_file_cf` interaction with `OptimisticTransactionDB`'s pending write set. | Accepted: ingest bypasses txn, parent-leaf write goes through txn, abort = wipe-and-restart. Documented in §6. |
| 4 | If someone re-runs InitChain on a non-fresh DB, ingest may collide with existing keys. | `IngestExternalFileOptions { allow_global_seqno: false }` + default `failed_move=true` causes overlap to error. Recovery = wipe DB and restart, which is the standard InitChain idempotency model. |
| 5 | Bake stage doubles image-build time even when `SDK_TEST_DATA=false`. | Gated on the build arg — `if [ "$SDK_TEST_DATA" = "true" ]` skips the cargo build entirely in production. |
| 6 | grovedb internal storage layout changes between bake and apply. | `grovedb_git_sha` hard error catches this. If you change grovedb, re-bake. |
| 7 | Snapshot file size (~150 MB at 500k notes). | Acceptable for a one-shot dev tool. Compressed inside the docker layer. |

## 12. Compatibility policy

**`format_version` bump policy:**
- Adding new fields to the header → `format_version` bump (new schema).
- Changing the meaning of an existing field → `format_version` bump.
- Changing how a field is computed (e.g. `bulk_state_root` derivation) →
  `format_version` bump.
- Bumping `grovedb_git_sha` alone does NOT bump `format_version`.

When `format_version` bumps, the apply side rejects older snapshots with
`FormatVersionMismatch`. There is no migration path — re-bake.

**`grovedb_git_sha` policy:**
- Embedded via build script: `vergen-git2` or similar produces a `const GROVEDB_GIT_SHA: [u8; 20]` at the grovedb crate root, exposed as `pub const fn grovedb_git_sha() -> [u8; 20]`.
- Bake writes its compiled-in value. Apply compares against its compiled-in
  value. Mismatch = `GrovedbRevMismatch`.
- Dirty-tree builds embed the SHA of HEAD (not the dirty state). This means
  during local dev, a bake against uncommitted changes can apply against a
  runtime built from those same uncommitted changes — acceptable, no
  cross-validation gap because the cmx-replay still catches semantic
  divergence.

## 13. Phasing

**This PR (single):**
- `grovedb-storage` additions: per-CF iteration on `StorageContext` (default + aux), `pub fn snapshot_db_options() -> Options`, and either a `pub fn raw_db()` accessor or a higher-level `pub fn apply_subtree_ingest()` helper on `GroveDb` (TBD during implementation — whichever exposes less surface).
- `grovedb` build-script for `grovedb_git_sha`.
- `rs-drive-abci::shielded_snapshot` module with `dump_shielded_subtree` + `apply_shielded_snapshot` + `read_header` + structured error enum.
- `snapshot-bake` CLI (one operation, hardcoded shielded-pool).
- InitChain hook reading `DRIVE_SHIELDED_SNAPSHOT` inside the existing transaction.
- Dockerfile bake stage gated on `SDK_TEST_DATA=true`.
- Dashmate `DRIVE_SHIELDED_SNAPSHOT` env-var forwarding.
- Remove `create_data_for_shielded_pool` runtime path entirely.
- Remove `CARGO_BUILD_PROFILE=release` workaround in `setup_local_network.sh`.
- All 11 tests above. Fuzz target. Equivalence test gated `#[ignore]`.

**Out of scope (follow-ups, if ever needed):**
- Other Element kinds (no genericity built in — if a second case appears, factor out shared pieces THEN, informed by two real cases).
- Multi-subtree snapshots.
- Snapshot signing.
- Streaming bake (for N > 5M).

## 14. Non-goals

- Not redesigning the runtime shielded seeder. The existing generator stays
  as the source of truth for how notes are produced; the snapshot is a
  frozen byte-for-byte snapshot of its output.
- Not introducing a new GroveDB storage format.
- Not abstracting over storage backends. Bake + ingest is RocksDB-only.

## 15. Phase 2 — frontier-less seeding for fast bake at large N

### 15.1 Motivation

Empirically measured bake times after the Phase-1 design landed:

| N | Bake wall-clock (Docker macOS, release) | Notes |
|---|---|---|
| 5_000 | ~30 s | proven via in-process roundtrip + devnet e2e |
| 500_000 | ~3 h 41 m | original Option-A measurement; matches "65 min native macOS × 3.4× Docker tax" |
| **1_000_000** | **>8 h, did not complete** | killed; estimated 4-5 h was wrong, Docker virt overhead worse than modelled |

The Sinsemilla frontier-append step is **75-85% of per-cmx cost** at all
N. For 1M notes, 999_992 of which are filler that no wallet owns, that's
~6-9 hours of Pallas math producing an anchor that nothing depends on.

### 15.2 Insight (grovedb PR #751, "claude/eloquent-margulis-8369cd")

Wallet sync verifies note membership via **BulkAppendTree chunk proofs**
authenticated by `bulk_state_root` (blake3-rooted), NOT via spend proofs
authenticated by the Sinsemilla anchor. So a chain whose anchor is
"junk" (e.g. reflects only the 8 owned cmx appends) still serves syncs
correctly — filler notes are recoverable, balances are recoverable, the
only thing missing is spend-proof authorization (which we never test).

The PR adds a test-only opt-out from the Sinsemilla path:

```rust
// On CommitmentTree (gated by `test-seeding-ct` feature in
// grovedb-commitment-tree; forwarded as `commitment_tree_test_seeding`
// from the grovedb crate).
pub fn append_raw_without_frontier(cmx, rho, payload) -> AppendResult;
pub fn append_many_without_frontier<I>(iter: I) -> BulkSeedSummary;
```

Both skip the Sinsemilla append entirely. The blake3-rooted BulkAppendTree
+ MMR + dense buffer + chunk blobs still get written. The frontier
storage at `__ct_data__` is left untouched (or reflects only the
non-skipped owned appends).

Additional perf win in the same PR: **MMR root cache** (commit
`7541294e`) — append becomes O(1) instead of O(N) for the MMR
incremental-update step. Stacks with the frontier skip.

### 15.2a Hard constraint discovered in crypto review (F1)

**`append_*_without_frontier` rejects a non-empty frontier.** PR #751
commit `c530e59a` (`grovedb-commitment-tree/src/commitment_tree/mod.rs:520-531`)
returns `InvalidData("frontier-less seeding requires an empty frontier")`
if `self.frontier.tree_size() != 0`.

This means we **cannot interleave** filler skip-path calls with owned
full-path calls. The bake order MUST be:

1. Bulk-seed ALL filler positions first via `append_many_without_frontier`
   while frontier is empty.
2. THEN loop the 8 owned through the normal full-Sinsemilla path
   (`commitment_tree_insert_op` / `append_raw`). At this point bulk
   positions [N-8, N) get appended; frontier picks up its first 8 cmx
   at frontier-positions 0..7 (independent counter from bulk).

**Consequence:** owned cmx live at bulk positions **[N-8, N)** —
contiguous tail — not striped across the tree the way the Phase 1
seeder placed them via `OwnedLayout::compute`. This shifts the
canonical placement; `OwnedLayout` must be updated accordingly (see
§15.3).

### 15.3 How we integrate

**Cherry-pick all 6 commits** of PR #751 onto our
`feat/snapshot-apply-public-api` branch (files don't overlap — they
touch `grovedb-commitment-tree` + `grovedb-bulk-append-tree`; we touch
`grovedb-storage` + `grovedb` + `grovedb/operations/commitment_tree.rs`'s
TAIL only).

**Feature name (corrected from earlier draft):** `test-seeding-ct` on
`grovedb-commitment-tree`, forwarded as `test-seeding-ct` on `grovedb`
itself (per commit `df492578` rename). Earlier sections referenced
`commitment_tree_test_seeding` — that name was dropped.

**Compile-time binding (per A2 review finding):** the runtime
`cfg(create_sdk_test_data)` gate and the Cargo `test-seeding-ct`
feature MUST always be set together. Add a `compile_error!` guard in
rs-drive-abci that fires if one is set without the other. This
prevents a build matrix bug where the cfg is on but the feature is
off (or vice versa), silently falling back to the slow path or
breaking the build trail.

**Refactor `OwnedLayout`** in
`packages/rs-drive-abci/.../create_genesis_state/test/shielded.rs`:

- Change `OwnedLayout::compute(total, owned_count)` to place owned at
  the **tail**: `wallet_a` at positions `[N - owned_count, N -
  owned_count + count_a)`, `wallet_b` at positions `[N - count_b, N)`.
- Update all in-process platform_tests (`seeded_pool_count_matches_*`,
  the address recovery tests at lines 595/701/722/789 per arch review)
  to expect the new positions.
- `wallet_at(pos)` mapping inverted accordingly.

**Update `seed_shielded_pool_with_config`** in the same file:

1. Generate the `Vec<SeededNote>` via the existing
   `generate_notes_for_test_wallets` (unchanged — only positions move).
2. Partition into `(filler, owned)` slices by ownership.
3. Open a single `CommitmentTree` via grovedb's storage context.
4. Call `commitment_tree.append_many_without_frontier(filler_iter)` —
   ALL filler in one bulk call. Frontier untouched.
5. Loop the 8 owned cmx through the existing
   `commitment_tree_insert_op` per-note path so they hit the full
   Sinsemilla append + parent-Merk leaf update.
6. **Post-bake assertion:** read back `commitment_tree_count()` and
   assert it equals `cfg.total_notes` (catches silent truncation from
   a mid-loop panic — F9 finding).

**Filler cmx — DEFER skipping rejection sampling (F6 caveat).** PR
#751 accepts non-Pallas cmx, but our review flagged that wallet
shardtree may compute `MerkleHashOrchard::from_bytes(cmx)` over filler
positions during sync (`packages/rs-platform-wallet/.../sync.rs:317-320`)
and reject if cmx is not a canonical Pallas base. **Empirically verify
before skipping the rejection sampler** — write a one-off test that
constructs a non-Pallas 32-byte sequence, feeds it as a filler cmx,
and asserts wallet sync still completes. If it fails, keep the
rejection sampler.

### 15.4 Expected bake wall-clock after integration

Per-note costs in release on this hardware (extrapolated from PR #751's
own benchmark + our measurements):

| Path | Cost per note |
|---|---|
| Old full append (Sinsemilla + BulkAppendTree) | ~7-8 ms |
| New `append_raw_without_frontier` (blake3 only) | ~1-2 μs |

For N=1_000_000 with 8 owned + 999_992 filler:

| Phase | Old | New |
|---|---|---|
| Filler appends | ~2.2 h | ~1-2 s |
| Owned appends | ~50 ms | ~50 ms |
| BulkAppendTree compactions (488 of them) | ~3 s | ~3 s |
| Frontier serialize/save | ~1 ms | ~1 ms |
| Dump (SST write) | ~5 s | ~5 s |
| **Total** | **~2.2 h** | **~10-30 s of CPU** |

Add docker/disk overhead: **end-to-end bake stage ~5-10 min total** for
N=1M (mostly fixed cargo/init/SST-finalize overhead, not Sinsemilla
work).

### 15.5 Consequences (must document in code + design)

- **Anchor is not a valid Orchard anchor.** After the seeded chain
  finalises InitChain, the recorded anchor at height 1 reflects only
  the 8 owned cmx, sitting at **frontier positions 0..7** (regardless
  of their bulk positions [N-8, N)). Production wallets attempting to
  construct spend proofs against this anchor will fail because their
  shardtree witness has cmx at bulk position N-8+k, but the on-chain
  anchor only knows it at frontier-position k — `merkle_path.root(cmx) != anchor`.
  This is acceptable because:
  - Devnet wallets in this test only sync, not spend.
  - The chain is gated by `cfg(create_sdk_test_data)` + `SDK_TEST_DATA=true`
    at image build — never reaches production.
  - **The Sinsemilla frontier and the BulkAppendTree maintain
    independent position counters** (grovedb-commitment-tree's
    `CommitmentFrontier::append` uses `incrementalmerkletree::Frontier`'s
    internal `position()`, NOT bulk's `total_count`). Confirmed by
    crypto review F2.
- **Add `tracing::warn!` at `record_shielded_pool_anchor_if_changed`
  call-site** noting the recorded anchor is NOT a valid Orchard spend
  anchor when the skip-path was used. (A6a finding.)
- **Wallet sync still works** because sync uses chunk proofs
  authenticated by the grovedb root hash, which transitively
  authenticates `bulk_state_root` (via the parent-Merk leaf's child
  hash = `compute_commitment_tree_state_root(sinsemilla, bulk_state)`).
  The wallet never compares `cmx` to the on-chain anchor — it appends
  to its own shardtree and uses `merkle_path.root(cmx)` from that
  local witness. Sync is anchor-agnostic. Confirmed by crypto review
  F4 (cited evidence: `packages/rs-drive-proof-verifier/src/proof.rs:2555-2565`,
  `packages/rs-platform-wallet/src/wallet/shielded/sync.rs:312-325`,
  `operations.rs:612-680`).
- **Cross-host determinism: same-host only.** Same RNG seed, same
  skip-path invocations, same blake3 output on the same host. Bake
  re-runs across hosts may legitimately produce different
  `__ct_data__` byte layouts (depends on MMR compaction timing,
  intermediate Merk operations). Phase 2 inherits the Phase 1
  same-host-only determinism property — does not weaken it.
- **Bake-time vs apply-time symmetry holds.** The snapshot stores
  whatever state the bake produced (including the "junk" anchor); apply
  ingests it verbatim; the combined-root cross-validation passes
  because both sides compute combined_root from the same loaded data.
  Confirmed by crypto review F5.
- **`combined_root` of a Phase-2 bake ≠ `combined_root` of a
  runtime-seeded chain** with the same config. This breaks the §10
  test #6 equivalence claim and the §8 "byte-identical to runtime"
  reasoning. Both have been scoped to Phase 1 above.

### 15.6 Test coverage to add

- **Modify** existing in-process roundtrip
  (`snapshot_dump_apply_preserves_anchor`) to assert anchor is
  non-empty (still true with owned-only frontier) AND wallet sync logic
  still finds the 8 owned notes through chunk proofs. **Drop the
  `#[ignore]` tag** — Phase 2 bake at N=1M is fast enough for default
  CI (A6e finding).
- **Retire** §10 test #6 (equivalence with runtime seeder). Replace
  with a "bake-side determinism" test: same config → same SST bytes
  across two bakes on the same host.
- **Add** F6-precondition test: feed a deliberately non-Pallas-canonical
  32-byte cmx as a filler, run wallet sync, assert it completes.
  Gates the "skip rejection sampling for filler" optimization. If this
  test fails, keep the rejection sampler.
- **Add** a test that **explicitly fails on attempted spend** against
  the Phase-2-seeded chain. Makes the "not spendable" caveat
  self-documenting and catches a regression where someone later tries
  to enable spends on test chains.
- **Add** a post-bake assertion that `commitment_tree_count() ==
  cfg.total_notes` to catch silent truncation from a panic mid-bake
  (F9 finding).
- **Benchmark test:** assert N=1_000_000 bake completes in <10 min
  wall-clock in CI (or `#[ignore]` if CI hardware is slow). Tracks the
  actual perf claim of Phase 2 against regressions.

### 15.7 Devnet visibility — progress logging

Independent of the perf fix: add periodic progress logs to
`seed_shielded_pool_with_config`. Currently the seeder is silent
between the "seeding ..." start and "dumping ..." end log lines, which
made the 8-hour blind wait extra painful. Emit every 30 s:

```
seed progress: appended 412480/1000000 (41.2%), elapsed 124s, rate 3326/s, ETA 176s
```

Keeps the bake observable even if some future change re-introduces a
slow loop somewhere.
- Not building a universal subtree-snapshot library. Shielded-specific by
  design.
