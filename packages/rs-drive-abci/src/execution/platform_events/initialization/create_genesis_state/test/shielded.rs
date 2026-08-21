//! Deterministic filler-note generator for the SDK genesis test-data seeder.
//!
//! Produces N filler notes — random valid Pallas-base `cmx` + random 32-byte
//! ρ + 216 random bytes of "ciphertext". No note is decryptable by any wallet
//! (ρ is not constrained to a valid `Nullifier`, so the SDK's compact
//! decryption short-circuits on the ρ-field check). For wallet-balance tests,
//! use real shielded-fund transitions post-genesis instead of seeded notes.
//!
//! All randomness comes from a single seeded `StdRng`. The GroveDB root hash
//! is byte-identical across hosts for a fixed `rng_seed`.

use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::Element;
use drive::grovedb::TransactionArg;
use drive::grovedb_path::SubtreePath;
use drive::grovedb_storage::{Storage, StorageBatch};
use grovedb_commitment_tree::{merkle_hash_from_bytes, CommitmentEntry, CommitmentTree, DashMemo};
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;

/// Block height at which we record the genesis post-seed anchor. Matches
/// production's first end-of-block anchor (`run_block_proposal` at the end of
/// block 1), so the seeded chain is root-identical to a production chain at
/// height 1 (modulo seeded note contents). See design doc §5.4.
const GENESIS_ANCHOR_HEIGHT: u64 = 1;

/// Dash Orchard wire size: 32 (epk) + 104 (enc_ciphertext, DashMemo) + 80 (out_ciphertext).
///
/// Pinned against the SDK's parser in
/// `packages/rs-sdk/src/platform/shielded/notes_sync/decrypt.rs`. If that file
/// changes its expected layout, the unit test below catches it.
pub const ENCRYPTED_NOTE_WIRE_LEN: usize = 32 + 104 + 80;
const _: () = assert!(ENCRYPTED_NOTE_WIRE_LEN == 216);

/// Configuration for the seeder.
///
/// The chain's `create_data_for_shielded_pool` uses [`Self::sdk_test_data`]
/// (hardcoded const), so every SDK_TEST_DATA devnet seeds the same N-note
/// pool regardless of operator env. Tests construct custom configs directly
/// to vary N.
#[derive(Debug, Clone)]
pub struct ShieldedSeedConfig {
    /// Total filler notes to seed.
    pub total_notes: u32,
    /// RNG seed; identical seed ⇒ identical root hash.
    pub rng_seed: u64,
}

impl Default for ShieldedSeedConfig {
    fn default() -> Self {
        Self {
            total_notes: 0,
            rng_seed: 0xDEAD_BEEF,
        }
    }
}

impl ShieldedSeedConfig {
    /// The hardcoded SDK_TEST_DATA seed config used at every devnet genesis.
    ///
    /// `total_notes = 1_000_000`, seed `0xDEAD_BEEF` is fixed so the GroveDB
    /// root hash is byte-identical across hosts.
    ///
    /// Wallet-balance UX no longer comes from seeded notes (the test wallet
    /// hardcoding was removed). For decryptable balance flows in tests, use
    /// real shielded-fund transitions post-genesis instead.
    ///
    /// At 1M notes the bake step takes ~5-15 min in the docker buildkit
    /// linux VM (release profile required — see
    /// `scripts/setup_local_network.sh::CARGO_BUILD_PROFILE`). Apply at
    /// runtime InitChain remains ~134 ms because it's a single SST ingest
    /// + parent-Merk leaf update regardless of N.
    pub const fn sdk_test_data() -> Self {
        Self {
            total_notes: 1_000_000,
            rng_seed: 0xDEAD_BEEF,
        }
    }
}

/// A single seeded filler note in the BulkAppendTree input shape.
#[derive(Debug, Clone)]
pub struct SeededNote {
    pub cmx: [u8; 32],
    /// On-wire `nullifier` field — this is ρ, *not* the spend-time revealed
    /// nullifier.
    pub rho: [u8; 32],
    /// 216 bytes: `epk(32) || enc_ciphertext(104) || out_ciphertext(80)`.
    pub encrypted_note: Vec<u8>,
}

/// Rejection-sample a valid Pallas base field element from the seeded RNG.
fn sample_valid_pallas_base(rng: &mut StdRng) -> [u8; 32] {
    loop {
        let mut bytes = [0u8; 32];
        rng.fill_bytes(&mut bytes);
        if merkle_hash_from_bytes(&bytes).is_some() {
            return bytes;
        }
    }
}

/// Build one filler note. ρ is intentionally random 32 bytes (not necessarily
/// a valid Pallas element) so the SDK's compact decryption short-circuits on
/// the field check — guarantees the filler is not decryptable by any wallet.
fn generate_filler_note(rng: &mut StdRng) -> SeededNote {
    let cmx = sample_valid_pallas_base(rng);
    let mut rho = [0u8; 32];
    rng.fill_bytes(&mut rho);
    let mut encrypted_note = vec![0u8; ENCRYPTED_NOTE_WIRE_LEN];
    rng.fill_bytes(&mut encrypted_note);
    SeededNote {
        cmx,
        rho,
        encrypted_note,
    }
}

/// Generate `count` filler notes from the seeded `rng`. Each call advances
/// `rng` deterministically, so chained calls across batches still produce
/// the byte-identical sequence that a single up-front call would have made.
pub fn generate_filler_batch(rng: &mut StdRng, count: usize) -> Vec<SeededNote> {
    (0..count).map(|_| generate_filler_note(rng)).collect()
}

/// Generate every seeded filler note up front. Equivalent to one
/// [`generate_filler_batch`] call sized to `cfg.total_notes`; only used by
/// tests that prefer the eager shape — the production seed path streams
/// batches via `generate_filler_batch` directly.
#[cfg(test)]
fn generate_notes(cfg: &ShieldedSeedConfig) -> Vec<SeededNote> {
    let mut rng = StdRng::seed_from_u64(cfg.rng_seed);
    generate_filler_batch(&mut rng, cfg.total_notes as usize)
}

impl<C> Platform<C> {
    /// Env-based entrypoint called by `create_sdk_test_data`. Reads
    /// `SHIELDED_SEED_*` and delegates to [`Self::seed_shielded_pool_with_config`].
    ///
    /// When `SHIELDED_SEED_TOTAL_NOTES = 0` (the default), no notes are seeded
    /// but the anchor recorder still runs — matching production's
    /// end-of-block-1 behaviour on an empty pool.
    ///
    /// **`None` transaction is tolerated**: production (`init_chain`) always
    /// passes a transaction, but several test helpers (notably
    /// `TestPlatformBuilder::set_genesis_state`) invoke `create_genesis_state`
    /// with `None`. In that case we skip both the seeding and the anchor
    /// recording — those tests aren't exercising the shielded pool, and the
    /// next test step that does will pass a tx through
    /// `seed_shielded_pool_with_config` explicitly.
    pub(super) fn create_data_for_shielded_pool(
        &self,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if transaction.is_none() {
            tracing::debug!(
                "create_data_for_shielded_pool: no transaction; skipping seeding \
                 (test path — production always supplies a tx)"
            );
            return Ok(());
        }

        // Fast-path: if `DRIVE_SHIELDED_SNAPSHOT` is set, load the
        // precomputed snapshot via SST ingest instead of running the (very
        // slow) runtime seeder. The snapshot file is produced offline by the
        // snapshot-bake binary against a chain whose `ShieldedSeedConfig`
        // matches what this build expects.
        //
        // Any failure (file missing, magic mismatch, version skew, checksum
        // failure, cross-validation drift) is FATAL — we surface the error
        // and InitChain fails loudly. No silent fallback to the seeder; the
        // operator is expected to fix the snapshot, not silently get a chain
        // with different state than they asked for.
        if let Ok(snapshot_path) = std::env::var("DRIVE_SHIELDED_SNAPSHOT") {
            let path = std::path::PathBuf::from(&snapshot_path);
            tracing::info!(
                snapshot_path = %path.display(),
                "create_data_for_shielded_pool: applying precomputed shielded snapshot"
            );
            let stats = crate::shielded_snapshot::apply_shielded_snapshot(
                &self.drive.grove,
                transaction,
                &path,
                platform_version,
            )
            .map_err(|e| {
                Error::Execution(ExecutionError::Conversion(format!(
                    "DRIVE_SHIELDED_SNAPSHOT apply failed: {e}"
                )))
            })?;
            tracing::info!(
                total_count = stats.total_count,
                combined_root = format!("{}", hex::encode(stats.combined_root)),
                "create_data_for_shielded_pool: snapshot applied"
            );
            // Record the anchor at height 1 so wallets see the same anchor
            // they would after a runtime seed. block_info is unused here
            // (record_shielded_pool_anchor_if_changed takes block_height
            // directly, and the bake step happens at genesis).
            let _ = block_info;
            let tx =
                transaction.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "create_data_for_shielded_pool snapshot path requires a transaction",
                )))?;
            self.drive
                .record_shielded_pool_anchor_if_changed(GENESIS_ANCHOR_HEIGHT, tx, platform_version)
                .map_err(Into::<Error>::into)
                .map(|_| ())?;
            return Ok(());
        }

        let cfg = ShieldedSeedConfig::sdk_test_data();
        tracing::info!(
            total_notes = cfg.total_notes,
            rng_seed = format!("0x{:x}", cfg.rng_seed),
            "create_data_for_shielded_pool: seeding SDK_TEST_DATA shielded pool"
        );
        self.seed_shielded_pool_with_config(&cfg, block_info, transaction, platform_version)
    }

    /// Seed the shielded pool with deterministic test data using the supplied
    /// config, then record the post-seed anchor at height 1.
    ///
    /// All randomness comes from `cfg.rng_seed`; identical config ⇒ identical
    /// GroveDB root hash. See design doc §5.3 + §5.4. Exposed for integration
    /// tests so they can pin specific configs without depending on env-var
    /// state.
    pub(super) fn seed_shielded_pool_with_config(
        &self,
        cfg: &ShieldedSeedConfig,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        tracing::info!(
            cfg_total_notes = cfg.total_notes,
            cfg_rng_seed = format!("0x{:x}", cfg.rng_seed),
            "seed_shielded_pool_with_config: entered"
        );
        if cfg.total_notes > 0 {
            tracing::info!(
                total_notes = cfg.total_notes,
                rng_seed = format!("0x{:x}", cfg.rng_seed),
                "seeding shielded pool with SDK test data (batched append_many_raw)"
            );
            let tx =
                transaction.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "seed_shielded_pool_with_config requires a transaction",
                )))?;

            // Read parent-Merk Element to get chunk_power + flags. The batched
            // seed path requires `total_count == 0` at entry — the running
            // `CommitmentTree` is opened against an empty subtree and we
            // accumulate `total_notes` appends on top.
            let pool_path_arr = drive::drive::shielded::paths::shielded_credit_pool_path();
            let leaf_key = &[drive::drive::shielded::paths::SHIELDED_NOTES_KEY];
            let parent_path = SubtreePath::from(pool_path_arr.as_slice());
            let element = self
                .drive
                .grove
                .get_raw(
                    parent_path.clone(),
                    leaf_key,
                    Some(tx),
                    &platform_version.drive.grove_version,
                )
                .value
                .map_err(|e| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(Box::leak(
                        format!("seed: get_raw parent leaf: {e}").into_boxed_str(),
                    )))
                })?;
            let (initial_total_count, chunk_power, flags) = match element {
                Element::CommitmentTree(tc, cp, f) => (tc, cp, f),
                _ => {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "seed: parent leaf is not Element::CommitmentTree",
                    )))
                }
            };
            assert_eq!(
                initial_total_count, 0,
                "batched seed requires an empty commitment tree"
            );

            // Open a CommitmentTree on the subtree's storage context. The
            // StorageBatch buffers every BulkAppendTree / frontier write and
            // is committed once at the end via `commit_multi_context_batch`
            // — same shape as the production `commitment_tree_insert` path.
            let subtree_path_segs: Vec<Vec<u8>> = pool_path_arr
                .iter()
                .map(|s| s.to_vec())
                .chain(std::iter::once(leaf_key.to_vec()))
                .collect();
            let subtree_path_refs: Vec<&[u8]> =
                subtree_path_segs.iter().map(|v| v.as_slice()).collect();
            let subtree_path = SubtreePath::from(subtree_path_refs.as_slice());

            let data_batch = StorageBatch::new();
            let storage_ctx = self
                .drive
                .grove
                .raw_storage()
                .get_transactional_storage_context(subtree_path, Some(&data_batch), tx)
                .unwrap();
            let mut ct = CommitmentTree::<_, DashMemo>::open(0, chunk_power, storage_ctx)
                .value
                .map_err(|e| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(Box::leak(
                        format!("seed: CommitmentTree::open: {e}").into_boxed_str(),
                    )))
                })?;

            // Batched seed via repeated `append_many_raw` (grovedb PR #751).
            // Each batch:
            //   1. Generates `BATCH_SIZE` filler notes from the seeded RNG.
            //   2. Calls `append_many_raw` — byte-for-byte equivalent to N ×
            //      `append_raw` per the upstream contract; flushes the MMR
            //      overlay internally.
            //   3. `ct.save()` persists the Sinsemilla frontier into the
            //      `data_batch` (still in-memory until the final commit).
            //   4. Logs the running anchor + bulk-state root so progress is
            //      observable and post-hoc replay is debuggable.
            //
            // The whole `data_batch` is committed once at the end. A crash
            // mid-bake loses every batch; durability per batch would require
            // a per-batch `commit_multi_context_batch`. For a bake binary
            // that lives in a single Docker stage producing an artifact, the
            // simpler shape is fine (re-run on failure).
            const BATCH_SIZE: usize = 10_000;
            let total = cfg.total_notes as usize;
            let bake_start = std::time::Instant::now();
            tracing::info!(
                total,
                batch_size = BATCH_SIZE,
                "seed: starting batched commitment-tree append"
            );
            let mut rng = StdRng::seed_from_u64(cfg.rng_seed);
            let mut appended_total = 0usize;
            let mut last_sinsemilla_root: [u8; 32] = [0u8; 32];
            let mut last_bulk_state_root: [u8; 32] = [0u8; 32];
            let mut batch_index = 0usize;
            while appended_total < total {
                let this_batch = std::cmp::min(BATCH_SIZE, total - appended_total);
                let batch_start = std::time::Instant::now();
                let batch_notes = generate_filler_batch(&mut rng, this_batch);
                let gen_elapsed = batch_start.elapsed();

                // Filler notes carry a dummy all-zero cv_net: they are not
                // decryptable by any wallet (ρ is not a valid Nullifier), so the
                // OVK-recovery use of cv_net never applies to them. The stored item
                // is `cmx || rho || cv_net || payload` (matching the production
                // commitment-tree insert path).
                let iter = batch_notes.into_iter().map(|n| CommitmentEntry {
                    cmx: n.cmx,
                    rho: n.rho,
                    cv_net: [0u8; 32],
                    payload: n.encrypted_note,
                });
                let append_result = ct
                    .append_many_raw(iter, &platform_version.drive.grove_version)
                    .value
                    .map_err(|e| {
                        Error::Execution(ExecutionError::CorruptedCodeExecution(Box::leak(
                            format!("seed: append_many_raw (batch {batch_index}): {e}")
                                .into_boxed_str(),
                        )))
                    })?;
                // Persist the Sinsemilla frontier per batch — cheap and
                // gives durable mid-bake checkpoints if we ever want to
                // resume from a crash. Mid-bake MMR `commit_mmr` is
                // intentionally NOT called here: per the upstream fix
                // `1340db71 fix(commitment-tree): don't commit_mmr inside
                // append_many_raw`, the MMR's compaction state is
                // accumulated in memory across `append_many_raw` calls and
                // flushed exactly once at the end. Calling `commit_mmr`
                // between batches corrupts the in-memory overlay (manifests
                // as "MMR get_root failed: Inconsistent store" on the next
                // call). One final `commit_mmr` follows the loop below.
                ct.save().value.map_err(|e| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(Box::leak(
                        format!("seed: ct.save (batch {batch_index}): {e}").into_boxed_str(),
                    )))
                })?;

                last_sinsemilla_root = append_result.sinsemilla_root;
                last_bulk_state_root = append_result.bulk_state_root;
                appended_total += this_batch;
                batch_index += 1;

                let total_elapsed = bake_start.elapsed();
                let rate = appended_total as f64 / total_elapsed.as_secs_f64().max(0.001);
                let remaining = total.saturating_sub(appended_total);
                let eta_secs = if rate > 0.0 {
                    (remaining as f64 / rate) as u64
                } else {
                    0
                };
                tracing::info!(
                    batch_index,
                    batch_size = this_batch,
                    appended = appended_total,
                    total,
                    pct = format!(
                        "{:.1}%",
                        (appended_total as f64 / total as f64) * 100.0
                    ),
                    elapsed_s = total_elapsed.as_secs(),
                    batch_elapsed_ms = batch_start.elapsed().as_millis() as u64,
                    gen_elapsed_ms = gen_elapsed.as_millis() as u64,
                    rate_per_s = format!("{:.0}", rate),
                    eta_s = eta_secs,
                    sinsemilla_root = %hex::encode(last_sinsemilla_root),
                    bulk_state_root = %hex::encode(last_bulk_state_root),
                    "seed batch complete"
                );
            }

            // Single MMR flush after every batch has appended. The MMR
            // overlay accumulates across `append_many_raw` calls and is
            // persisted only here. See the in-loop comment above and the
            // upstream fix `1340db71` for the rationale.
            ct.commit_mmr().map_err(|e| {
                Error::Execution(ExecutionError::CorruptedCodeExecution(Box::leak(
                    format!("seed: ct.commit_mmr (final): {e}").into_boxed_str(),
                )))
            })?;

            let combined_root = grovedb_commitment_tree::compute_commitment_tree_state_root(
                &last_sinsemilla_root,
                &last_bulk_state_root,
            );
            tracing::info!(
                total,
                batches = batch_index,
                elapsed_s = bake_start.elapsed().as_secs(),
                combined_root = %hex::encode(combined_root),
                "seed: all batches complete"
            );
            drop(ct);

            // Commit the subtree's data_batch through the transaction —
            // makes all the BulkAppendTree / frontier writes visible to
            // subsequent reads (including replace_subtree_root's own batch).
            self.drive
                .grove
                .raw_storage()
                .commit_multi_context_batch(data_batch, Some(tx))
                .value
                .map_err(|e| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(Box::leak(
                        format!("seed: commit_multi_context_batch: {e}").into_boxed_str(),
                    )))
                })?;

            // --- Update parent Merk leaf with the new state ---
            let new_element = drive::grovedb::Element::new_commitment_tree(
                u64::from(cfg.total_notes),
                chunk_power,
                flags,
            );
            self.drive
                .grove
                .replace_subtree_root(
                    parent_path,
                    leaf_key,
                    new_element,
                    combined_root,
                    Some(tx),
                    &platform_version.drive.grove_version,
                )
                .value
                .map_err(|e| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(Box::leak(
                        format!("seed: replace_subtree_root: {e}").into_boxed_str(),
                    )))
                })?;

            // Post-bake assertion — catches silent truncation from a panic
            // mid-bake (per design doc §15.6 F9).
            let mut drive_ops = vec![];
            let count_after =
                self.drive
                    .shielded_pool_notes_count(Some(tx), &mut drive_ops, platform_version)?;
            assert_eq!(
                count_after,
                u64::from(cfg.total_notes),
                "seed: post-bake count mismatch (expected {}, got {})",
                cfg.total_notes,
                count_after
            );

            // block_info is unused on this fast path (no per-note state
            // transitions, no fee accounting) — accepted as a signature
            // artifact for compatibility.
            let _ = block_info;
        }

        // Always record the post-seed anchor at height 1 — matches production's
        // first end-of-block anchor. With cfg.total_notes == 0 this records the
        // empty-tree Sinsemilla root, which is the same value production
        // records at end of block 1 against an empty pool. See design doc §5.4
        // for why one anchor suffices (wallet creates a single checkpoint at
        // post-sync tree size).
        let tx = transaction.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
            "create_data_for_shielded_pool requires a transaction",
        )))?;
        self.drive
            .record_shielded_pool_anchor_if_changed(GENESIS_ANCHOR_HEIGHT, tx, platform_version)
            .map_err(Error::Drive)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn small_cfg() -> ShieldedSeedConfig {
        ShieldedSeedConfig {
            total_notes: 16,
            rng_seed: 0xDEAD_BEEF,
        }
    }

    #[test]
    fn generate_notes_count_matches_total() {
        let cfg = small_cfg();
        let notes = generate_notes(&cfg);
        assert_eq!(notes.len(), cfg.total_notes as usize);
    }

    #[test]
    fn generate_notes_filler_ciphertext_size_pinned_to_216() {
        let cfg = small_cfg();
        let notes = generate_notes(&cfg);
        for n in &notes {
            assert_eq!(
                n.encrypted_note.len(),
                ENCRYPTED_NOTE_WIRE_LEN,
                "all encrypted_note payloads must be 216 bytes — matches \
                 packages/rs-sdk/src/platform/shielded/notes_sync/decrypt.rs"
            );
        }
    }

    #[test]
    fn generate_notes_is_deterministic() {
        let cfg = small_cfg();
        let a = generate_notes(&cfg);
        let b = generate_notes(&cfg);
        assert_eq!(a.len(), b.len());
        for (i, (na, nb)) in a.iter().zip(b.iter()).enumerate() {
            assert_eq!(na.cmx, nb.cmx, "cmx differs at position {}", i);
            assert_eq!(na.rho, nb.rho, "rho differs at position {}", i);
            assert_eq!(
                na.encrypted_note, nb.encrypted_note,
                "encrypted_note differs at position {}",
                i
            );
        }
    }

    #[test]
    fn generate_notes_changes_with_different_seed() {
        let cfg_a = ShieldedSeedConfig {
            rng_seed: 1,
            ..small_cfg()
        };
        let cfg_b = ShieldedSeedConfig {
            rng_seed: 2,
            ..small_cfg()
        };
        let a = generate_notes(&cfg_a);
        let b = generate_notes(&cfg_b);
        // At least one cmx differs (the first one — different RNG stream).
        assert!(a.iter().zip(b.iter()).any(|(na, nb)| na.cmx != nb.cmx));
    }

    /// The batched generator must produce byte-identical output to a single
    /// up-front call. Otherwise the seeded GroveDB state diverges between
    /// "one big append_many_raw" and "N × append_many_raw(BATCH_SIZE)".
    #[test]
    fn batched_generator_matches_single_call() {
        let cfg = ShieldedSeedConfig {
            total_notes: 100,
            rng_seed: 0xDEAD_BEEF,
        };
        let one_shot = generate_notes(&cfg);

        let mut rng = StdRng::seed_from_u64(cfg.rng_seed);
        let mut batched: Vec<SeededNote> = Vec::with_capacity(100);
        for _ in 0..10 {
            batched.extend(generate_filler_batch(&mut rng, 10));
        }
        assert_eq!(one_shot.len(), batched.len());
        for (i, (a, b)) in one_shot.iter().zip(batched.iter()).enumerate() {
            assert_eq!(a.cmx, b.cmx, "cmx differs at {i}");
            assert_eq!(a.rho, b.rho, "rho differs at {i}");
            assert_eq!(a.encrypted_note, b.encrypted_note, "ct differs at {i}");
        }
    }

    /// Sanity: 1k filler notes have unique cmx values with overwhelming
    /// probability (Pallas base field is enormous, collisions are
    /// cryptographically negligible). A regression where cmx is no longer
    /// drawn freshly per call would surface as a duplicate here.
    #[test]
    fn generated_cmx_values_are_unique() {
        let cfg = ShieldedSeedConfig {
            total_notes: 1024,
            rng_seed: 0xDEAD_BEEF,
        };
        let notes = generate_notes(&cfg);
        let mut seen: HashSet<[u8; 32]> = HashSet::with_capacity(notes.len());
        for n in &notes {
            assert!(seen.insert(n.cmx), "duplicate cmx in generator output");
        }
    }
}

#[cfg(test)]
mod platform_tests {
    //! End-to-end tests that drive `seed_shielded_pool_with_config` against a
    //! real `Platform` instance. The pure-function tests for the note generator
    //! live in the sibling `tests` module above; these tests verify the
    //! integration with Drive (count, anchor, determinism) — i.e. that
    //! production's `apply_drive_operations` ⇒ `commitment_tree_insert_op` path
    //! ends up consistent with what the generator produced.

    use super::*;
    use crate::config::PlatformConfig;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use drive::drive::shielded::paths::{shielded_credit_pool_path, SHIELDED_NOTES_KEY};
    use grovedb_commitment_tree::EMPTY_SINSEMILLA_ROOT;

    /// Reduced default for integration tests — smaller is faster and still
    /// exercises the batched seed path on small N.
    fn integration_cfg() -> ShieldedSeedConfig {
        ShieldedSeedConfig {
            total_notes: 16,
            rng_seed: 0xDEAD_BEEF,
        }
    }

    /// `set_genesis_state` runs `create_sdk_test_data` under the cfg flag, which
    /// errors unless the platform is on the `Regtest` network. The default
    /// `TestPlatformBuilder::new()` config is mainnet, so every test in this
    /// module has to switch to regtest before calling `set_genesis_state`.
    fn build_regtest_platform(
    ) -> crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike> {
        TestPlatformBuilder::new()
            .with_config(PlatformConfig::default_local())
            .build_with_mock_rpc()
            .set_genesis_state()
    }

    /// Read the current Sinsemilla anchor for the credit shielded pool.
    fn read_current_anchor<C>(
        platform: &Platform<C>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> [u8; 32] {
        let pool_path = shielded_credit_pool_path();
        platform
            .drive
            .grove
            .commitment_tree_anchor(
                &pool_path,
                &[SHIELDED_NOTES_KEY],
                transaction,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("commitment_tree_anchor")
            .to_bytes()
    }

    /// Build a fresh test platform, seed with `cfg`, return the post-seed Sinsemilla
    /// anchor read inside the same transaction.
    fn build_and_seed(cfg: &ShieldedSeedConfig, platform_version: &PlatformVersion) -> [u8; 32] {
        let platform = build_regtest_platform();
        let tx = platform.drive.grove.start_transaction();
        platform
            .seed_shielded_pool_with_config(cfg, &BlockInfo::default(), Some(&tx), platform_version)
            .expect("seed must succeed");
        read_current_anchor(&platform.platform, Some(&tx), platform_version)
    }

    #[test]
    fn empty_config_leaves_pool_at_empty_sinsemilla_root() {
        // After `set_genesis_state`, the env-based call ran with total_notes=0
        // (no SHIELDED_SEED_* envs set in this test), so the live Sinsemilla
        // frontier must equal the well-known empty-tree root.
        let platform_version = PlatformVersion::latest();
        let platform = build_regtest_platform();
        let tx = platform.drive.grove.start_transaction();
        let anchor = read_current_anchor(&platform, Some(&tx), platform_version);
        assert_eq!(anchor, EMPTY_SINSEMILLA_ROOT);
    }

    #[test]
    fn seeded_pool_count_matches_total_notes() {
        let platform_version = PlatformVersion::latest();
        let platform = build_regtest_platform();
        let tx = platform.drive.grove.start_transaction();
        let cfg = integration_cfg();
        platform
            .seed_shielded_pool_with_config(
                &cfg,
                &BlockInfo::default(),
                Some(&tx),
                platform_version,
            )
            .expect("seed");

        let mut drive_ops = vec![];
        let count = platform
            .drive
            .shielded_pool_notes_count(Some(&tx), &mut drive_ops, platform_version)
            .expect("shielded_pool_notes_count");
        assert_eq!(count, u64::from(cfg.total_notes));
    }

    #[test]
    fn seeded_anchor_differs_from_empty_root() {
        let platform_version = PlatformVersion::latest();
        let cfg = integration_cfg();
        let anchor = build_and_seed(&cfg, platform_version);
        assert_ne!(
            anchor, EMPTY_SINSEMILLA_ROOT,
            "post-seed anchor must differ from the empty-tree root"
        );
    }

    #[test]
    fn seeding_with_same_config_is_byte_identical_across_platforms() {
        // The load-bearing determinism test: two fresh platforms, same config,
        // must produce byte-identical Sinsemilla anchors. If this ever fails,
        // some part of the generator is consuming `OsRng` / `thread_rng()`
        // instead of the seeded RNG, or a transitive dep changed Pallas-field
        // semantics under us.
        let platform_version = PlatformVersion::latest();
        let cfg = integration_cfg();
        let anchor1 = build_and_seed(&cfg, platform_version);
        let anchor2 = build_and_seed(&cfg, platform_version);
        assert_eq!(
            anchor1, anchor2,
            "same config must produce byte-identical anchors across runs"
        );
    }

    #[test]
    fn different_rng_seeds_produce_different_anchors() {
        let platform_version = PlatformVersion::latest();
        let cfg_1 = ShieldedSeedConfig {
            rng_seed: 1,
            ..integration_cfg()
        };
        let cfg_2 = ShieldedSeedConfig {
            rng_seed: 2,
            ..integration_cfg()
        };
        let a1 = build_and_seed(&cfg_1, platform_version);
        let a2 = build_and_seed(&cfg_2, platform_version);
        assert_ne!(a1, a2);
    }

    #[test]
    fn seeding_zero_notes_keeps_pool_at_empty_root() {
        // Explicit zero-notes config must produce the empty-tree root, matching
        // the no-config path. Pins the §5.4 claim that N=0 is a no-op for the
        // commitment tree but still triggers the anchor recorder.
        let platform_version = PlatformVersion::latest();
        let cfg = ShieldedSeedConfig {
            total_notes: 0,
            ..ShieldedSeedConfig::default()
        };
        let anchor = build_and_seed(&cfg, platform_version);
        assert_eq!(anchor, EMPTY_SINSEMILLA_ROOT);
    }

    /// Native bake-feasibility benchmark — runs the FULL seeder (default N=500k,
    /// overridable via `BENCH_N`) outside any Docker file-share layer and prints
    /// wall-clock to stderr. The grovedb tempdir lands in `$TMPDIR`, which on
    /// macOS is APFS (fast native fsync) and on linux CI is typically tmpfs.
    ///
    /// Compare-against:
    /// - macOS Docker Desktop file-share, N=500k, release: ~3h 41m (measured).
    /// - Same host natively (this test), N=500k, release: <expected: 2–10 min>.
    /// - Linux buildkit VM (where the bake stage would run), N=500k, release:
    ///   should be at most this number, usually faster.
    ///
    /// Hard-fail if the seed doesn't complete in 30 min — that's the threshold
    /// past which the bake approach itself is in trouble.
    ///
    /// Run with:
    /// ```bash
    /// cargo test -p drive-abci --release --features create_sdk_test_data \
    ///   bench_native_seed_full -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "long-running bake-feasibility benchmark, see fn docs"]
    fn bench_native_seed_full() {
        let n: u32 = std::env::var("BENCH_N")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(500_000);

        let platform_version = PlatformVersion::latest();
        let platform = build_regtest_platform();

        let cfg = ShieldedSeedConfig {
            total_notes: n,
            rng_seed: 0xDEAD_BEEF,
        };

        let tempdir_path = platform.tempdir.path().display().to_string();
        eprintln!(
            "bench_native_seed_full: N={} cfg={:?} tempdir={}",
            n, cfg, tempdir_path,
        );

        let tx = platform.drive.grove.start_transaction();
        let start = std::time::Instant::now();
        platform
            .seed_shielded_pool_with_config(
                &cfg,
                &BlockInfo::default(),
                Some(&tx),
                platform_version,
            )
            .expect("seed_shielded_pool_with_config must succeed");
        let seed_elapsed = start.elapsed();

        let commit_start = std::time::Instant::now();
        tx.commit().expect("commit");
        let commit_elapsed = commit_start.elapsed();

        let total = seed_elapsed + commit_elapsed;
        eprintln!(
            "bench_native_seed_full: N={} seed={:.2?} commit={:.2?} total={:.2?} \
             ({:.1} notes/sec)",
            n,
            seed_elapsed,
            commit_elapsed,
            total,
            n as f64 / total.as_secs_f64(),
        );

        // Anchor sanity check so we know the work was real, not a noop.
        let anchor = read_current_anchor(&platform, None, platform_version);
        assert_ne!(
            anchor, EMPTY_SINSEMILLA_ROOT,
            "post-seed anchor must differ from empty (proves seeding actually ran)",
        );

        assert!(
            total.as_secs() < 30 * 60,
            "seed took {:?} for N={}; bake approach assumes native seeding is well \
             under 30 min — investigate if this fails",
            total,
            n,
        );
    }

    /// Empirically verify the snapshot design's data-location claim — that the
    /// shielded commitment-tree subtree's RocksDB state lives entirely in the
    /// `default` + `aux` CFs under a deterministic subtree prefix. If
    /// `roots` or `meta` CFs ALSO have entries for our subtree prefix, the
    /// snapshot file format needs to include them and `apply_shielded_snapshot`
    /// must ingest into 4 CFs not 2.
    ///
    /// What this proves:
    /// 1. The subtree prefix is path-deterministic (built via
    ///    `RocksDbStorage::build_prefix` from `[shielded_credit_pool_path()..,
    ///    SHIELDED_NOTES_KEY]`).
    /// 2. After seeding N notes, the CFs with non-zero key counts under that
    ///    prefix tell us which CFs the dump must include.
    /// 3. Counts scale with N as expected (default CF has chunks + buffer + MMR
    ///    + META; aux CF has exactly 1 entry, the Sinsemilla frontier).
    #[test]
    #[ignore = "raw-rocksdb introspection; long-ish setup"]
    fn dump_only_default_and_aux_cfs_under_shielded_subtree_prefix() {
        use grovedb_path::SubtreePath;
        use grovedb_storage::rocksdb_storage::RocksDbStorage;
        use rocksdb::{ColumnFamilyDescriptor, OptimisticTransactionDB, Options};

        let platform_version = PlatformVersion::latest();
        let temp = build_regtest_platform();

        // Seed a small N so the test is fast but still exercises every key
        // family the BulkAppendTree writes (META, ≥1 buffer entry, ≥1 chunk if
        // N ≥ 2^chunk_power).
        let cfg = ShieldedSeedConfig {
            total_notes: 4096, // > one chunk at chunk_power=11 → exercises e/m keys
            rng_seed: 0xDEAD_BEEF,
        };
        let tx = temp.drive.grove.start_transaction();
        temp.seed_shielded_pool_with_config(
            &cfg,
            &BlockInfo::default(),
            Some(&tx),
            platform_version,
        )
        .expect("seed");
        tx.commit().expect("commit");
        let anchor = read_current_anchor(&temp, None, platform_version);
        assert_ne!(anchor, EMPTY_SINSEMILLA_ROOT);

        // Compute the shielded subtree's deterministic prefix using grovedb's
        // own helper — same algorithm production uses, no reimplementation.
        let pool = drive::drive::shielded::paths::shielded_credit_pool_path_vec();
        let mut path_segments: Vec<Vec<u8>> = pool;
        path_segments.push(vec![drive::drive::shielded::paths::SHIELDED_NOTES_KEY]);
        let path = SubtreePath::from(path_segments.as_slice());
        let prefix = RocksDbStorage::build_prefix(path).unwrap();
        let prefix_bytes: [u8; 32] = prefix.into();
        eprintln!("subtree_prefix = {}", hex::encode(prefix_bytes));

        // Close the Platform so we can take an exclusive lock on RocksDB.
        // Destructure to keep the TempDir alive while the Platform's GroveDb
        // handle is dropped — otherwise dropping the whole TempPlatform also
        // drops the TempDir and deletes the directory underneath us.
        let crate::test::helpers::setup::TempPlatform {
            platform: pf,
            tempdir,
        } = temp;
        let tempdir_path = tempdir.path().to_path_buf();
        drop(pf);

        // Open the same DB directly via rocksdb. CF names match grovedb's
        // pub(crate) consts in `grovedb-storage/src/rocksdb_storage/storage.rs`:
        // "default" is the implicit primary CF, the other three are named.
        // If those names ever change in grovedb, this test must be updated;
        // the production snapshot code will need the same update.
        let cf_names = ["default", "aux", "roots", "meta"];
        let opts = {
            let mut o = Options::default();
            o.create_if_missing(false);
            o.create_missing_column_families(false);
            o
        };
        let cf_descs: Vec<ColumnFamilyDescriptor> = cf_names
            .iter()
            .map(|n| ColumnFamilyDescriptor::new(*n, Options::default()))
            .collect();
        let db: OptimisticTransactionDB =
            OptimisticTransactionDB::open_cf_descriptors(&opts, &tempdir_path, cf_descs)
                .expect("open A rocksdb");

        // For each CF, count keys whose first 32 bytes equal the subtree prefix.
        // Capture the first few key bytes too so a regression has actionable
        // info, not just a count mismatch.
        let mut per_cf_counts: Vec<(&str, usize, Vec<Vec<u8>>)> = vec![];
        for cf_name in cf_names {
            let cf = db.cf_handle(cf_name).unwrap_or_else(|| {
                panic!("missing CF {cf_name} — grovedb's CF layout may have changed")
            });
            let mut iter = db.raw_iterator_cf(&cf);
            iter.seek(&prefix_bytes[..]);
            let mut count = 0usize;
            let mut samples: Vec<Vec<u8>> = vec![];
            while iter.valid() {
                let k = match iter.key() {
                    Some(k) => k,
                    None => break,
                };
                if !k.starts_with(&prefix_bytes) {
                    break;
                }
                count += 1;
                if samples.len() < 5 {
                    samples.push(k.to_vec());
                }
                iter.next();
            }
            eprintln!(
                "CF {:>7}: {} keys under subtree prefix (first {} sampled)",
                cf_name,
                count,
                samples.len(),
            );
            for s in &samples {
                eprintln!("  sample key (len={}): {}", s.len(), hex::encode(s));
            }
            per_cf_counts.push((cf_name, count, samples));
        }
        drop(db);

        let counts: std::collections::HashMap<&str, usize> =
            per_cf_counts.iter().map(|(n, c, _)| (*n, *c)).collect();

        let default_count = *counts.get("default").unwrap();
        let aux_count = *counts.get("aux").unwrap();
        let roots_count = *counts.get("roots").unwrap();
        let meta_count = *counts.get("meta").unwrap();

        // Empirical claim: the shielded commitment-tree subtree's ENTIRE state
        // — DenseFixedSizedMerkleTree buffer nodes (2-byte BE position keys),
        // chunk MMR nodes, META, and the Sinsemilla frontier
        // (`COMMITMENT_TREE_DATA_KEY`) — lives in the **default CF only**. The
        // frontier is written via `put(...)` not `put_aux(...)` despite the
        // misleading internal naming.
        //
        // This is a simplification of the original snapshot design which
        // assumed default + aux. With only default CF in scope, the snapshot
        // format reduces to "one SST per subtree" not "two SSTs per subtree".
        assert!(
            default_count > 0,
            "default CF must have entries under subtree prefix; \
             snapshot design is broken if this fails"
        );
        assert_eq!(
            aux_count, 0,
            "aux CF unexpectedly has {aux_count} entries — the Sinsemilla frontier \
             was relocated to aux somewhere. Re-verify storage layout in \
             grovedb-commitment-tree::commitment_tree::mod.rs (search for `put_aux`)"
        );
        assert_eq!(
            roots_count, 0,
            "roots CF unexpectedly has {roots_count} entries under shielded subtree prefix; \
             snapshot dump must include the roots CF (currently doesn't)"
        );
        assert_eq!(
            meta_count, 0,
            "meta CF unexpectedly has {meta_count} entries under shielded subtree prefix; \
             snapshot dump must include the meta CF (currently doesn't)"
        );
    }

    /// End-to-end roundtrip: dump the shielded subtree from platform A into a
    /// snapshot file, apply it to a fresh platform B, assert anchor_B ==
    /// anchor_A. Proves that `dump_shielded_subtree` + `apply_shielded_snapshot`
    /// preserve the Sinsemilla anchor (which is what wallet sync reads to
    /// verify membership proofs).
    ///
    /// Uses N=30_000 — three batches at `BATCH_SIZE = 10_000`, so the
    /// seeder exercises the inter-batch MMR-overlay handoff. The upstream
    /// fix `1340db71 fix(commitment-tree): don't commit_mmr inside
    /// append_many_raw` (plus our matching loop restructure that defers
    /// `commit_mmr` to the end) is what makes multi-batch work; this test
    /// pins that contract.
    #[test]
    fn snapshot_dump_apply_preserves_anchor() {
        let platform_version = PlatformVersion::latest();

        // --- A: build, seed, capture anchor ---
        let platform_a = build_regtest_platform();
        let cfg = ShieldedSeedConfig {
            total_notes: 30_000,
            rng_seed: 0xDEAD_BEEF,
        };
        let tx_a = platform_a.drive.grove.start_transaction();
        platform_a
            .seed_shielded_pool_with_config(
                &cfg,
                &BlockInfo::default(),
                Some(&tx_a),
                platform_version,
            )
            .expect("seed A");
        tx_a.commit().expect("commit A");
        let anchor_a = read_current_anchor(&platform_a, None, platform_version);
        assert_ne!(
            anchor_a, EMPTY_SINSEMILLA_ROOT,
            "A must have non-empty anchor"
        );
        eprintln!("anchor_a = {}", hex::encode(anchor_a));

        // --- Dump A to a temporary snapshot file ---
        let dump_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_path = dump_dir.path().join("shielded-pool.snap");
        let dump_stats = crate::shielded_snapshot::dump_shielded_subtree(
            &platform_a.drive.grove,
            None,
            &snapshot_path,
            platform_version,
        )
        .expect("dump");
        eprintln!(
            "dump: total_count={} key_count={} sst_bytes={}",
            dump_stats.total_count, dump_stats.key_count, dump_stats.sst_bytes,
        );
        assert_eq!(dump_stats.total_count, u64::from(cfg.total_notes));
        // The DenseFixedSizedMerkleTree + chunk MMR + frontier produces O(N)
        // keys; for N=5000 we expect roughly 2000+ keys (sanity: must be > 0).
        assert!(dump_stats.key_count > 0);

        // --- B: build (parent skeleton present, shielded pool EMPTY) ---
        let platform_b = build_regtest_platform();
        let pre_anchor_b = read_current_anchor(&platform_b, None, platform_version);
        assert_eq!(
            pre_anchor_b, EMPTY_SINSEMILLA_ROOT,
            "B should start with empty shielded pool"
        );

        // --- Apply the snapshot to B ---
        let apply_stats = crate::shielded_snapshot::apply_shielded_snapshot(
            &platform_b.drive.grove,
            None,
            &snapshot_path,
            platform_version,
        )
        .expect("apply");
        eprintln!(
            "apply: total_count={} combined_root={}",
            apply_stats.total_count,
            hex::encode(apply_stats.combined_root),
        );

        // --- Verify: anchor_B equals anchor_A ---
        let anchor_b = read_current_anchor(&platform_b, None, platform_version);
        eprintln!("anchor_b = {}", hex::encode(anchor_b));
        assert_eq!(
            anchor_b, anchor_a,
            "anchor must match after dump + apply roundtrip"
        );

        // --- Also verify: total_count via shielded_pool_notes_count ---
        let mut drive_ops = vec![];
        let count_b = platform_b
            .drive
            .shielded_pool_notes_count(None, &mut drive_ops, platform_version)
            .expect("count B");
        assert_eq!(
            count_b,
            u64::from(cfg.total_notes),
            "B's note count must match A's seed config"
        );
    }

    /// Reproduces the production devnet InitChain failure:
    ///   `DRIVE_SHIELDED_SNAPSHOT apply failed: grovedb: ingest_subtree_sst:
    ///    ... ingest_subtree_sst(default, /tmp/drv-shielded-snapshot-*.sst)
    ///    failed: Invalid argument: Global seqno is required, but disabled`
    ///
    /// Root cause: `apply_shielded_snapshot`'s SST ingest commits DIRECTLY to
    /// the underlying RocksDB (`ingest_external_file_cf`), bypassing the
    /// InitChain GroveDB transaction — and nothing wipes the DB on InitChain.
    /// The genesis transaction is committed only at the end of finalizing
    /// block 1, so if InitChain never reaches that commit (any genesis/block-1
    /// failure, a drive-abci restart, or a Tenderdash InitChain retry) the
    /// transaction rolls back but the **already-ingested SST keys persist** in
    /// the committed `default` CF. The next InitChain attempt re-runs the
    /// ingest over the now-committed keys; the SST's key range overlaps
    /// existing committed keys, so RocksDB needs to assign a global sequence
    /// number, which grovedb's `ingest_subtree_sst` forbids
    /// (`allow_global_seqno=false`) → "Global seqno is required, but disabled",
    /// and the devnet can never initialize.
    ///
    /// Applying a deterministic genesis snapshot must therefore be IDEMPOTENT:
    /// after a failed attempt's transaction rolls back, the next attempt must
    /// succeed even though the orphaned ingest survived the rollback.
    ///
    /// This test models the production scenario exactly:
    ///   1. apply the snapshot WITH a transaction (`Some(tx1)`) — the ingest
    ///      writes straight to RocksDB, the parent-leaf patch goes into `tx1`;
    ///   2. DROP `tx1` without committing — the InitChain failure path. This
    ///      rolls back the parent-leaf patch (and, in production, all genesis
    ///      writes) but NOT the ingest, which never went through `tx1`;
    ///   3. apply again WITH a fresh transaction (`Some(tx2)`) — the retry.
    ///
    /// The key point (and the answer to "shouldn't rollback restore an empty
    /// DB?"): rollback would restore an empty DB only if the ingest were
    /// transactional. It is not — so after step 2 the `default` CF still holds
    /// the orphaned SST keys, and step 3's re-ingest overlaps them.
    ///
    /// Would have caught the production failure in CI:
    ///   ✖ step 3 errors with "Global seqno is required, but disabled" before
    ///     the fix,
    ///   ✔ step 3 succeeds (safe no-op) after.
    #[test]
    fn snapshot_reapply_is_idempotent_under_initchain_retry() {
        let platform_version = PlatformVersion::latest();

        // --- A: seed a small pool and dump it to a snapshot file ---
        let platform_a = build_regtest_platform();
        let cfg = ShieldedSeedConfig {
            total_notes: 5_000,
            rng_seed: 0xDEAD_BEEF,
        };
        let tx_a = platform_a.drive.grove.start_transaction();
        platform_a
            .seed_shielded_pool_with_config(
                &cfg,
                &BlockInfo::default(),
                Some(&tx_a),
                platform_version,
            )
            .expect("seed A");
        tx_a.commit().expect("commit A");
        let anchor_a = read_current_anchor(&platform_a, None, platform_version);
        assert_ne!(anchor_a, EMPTY_SINSEMILLA_ROOT);

        let dump_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_path = dump_dir.path().join("shielded-pool.snap");
        crate::shielded_snapshot::dump_shielded_subtree(
            &platform_a.drive.grove,
            None,
            &snapshot_path,
            platform_version,
        )
        .expect("dump");

        // --- B, attempt 1: apply inside a transaction, then ROLL BACK ---
        // Models an InitChain that ingests the snapshot but never reaches the
        // block-1 commit (a later genesis/block failure, a restart, or a
        // Tenderdash InitChain retry).
        let platform_b = build_regtest_platform();
        {
            let tx1 = platform_b.drive.grove.start_transaction();
            crate::shielded_snapshot::apply_shielded_snapshot(
                &platform_b.drive.grove,
                Some(&tx1),
                &snapshot_path,
                platform_version,
            )
            .expect("first apply (inside tx1) must succeed");
            // Drop tx1 WITHOUT committing — the failure/rollback path. The
            // parent-leaf patch (in tx1) is discarded; the ingest is NOT.
            drop(tx1);
        }

        // --- B, attempt 2: the retry, in a fresh transaction ---
        // The orphaned ingest from attempt 1 is still committed in the
        // `default` CF, so this re-ingest overlaps it — the exact production
        // failure ("Global seqno is required, but disabled").
        let tx2 = platform_b.drive.grove.start_transaction();
        let retry = crate::shielded_snapshot::apply_shielded_snapshot(
            &platform_b.drive.grove,
            Some(&tx2),
            &snapshot_path,
            platform_version,
        );
        assert!(
            retry.is_ok(),
            "re-applying a deterministic genesis snapshot after a rolled-back \
             attempt must be idempotent (InitChain retry path), but it failed: \
             {retry:?}"
        );
        tx2.commit().expect("commit retry tx");

        let anchor_b = read_current_anchor(&platform_b, None, platform_version);
        assert_eq!(
            anchor_b, anchor_a,
            "anchor must match after an idempotent re-apply"
        );
    }

    // Real InitChain hook coverage happens via the dashmate devnet flow
    // (see docs/genesis-snapshot-design.md §13 e2e). An in-process equivalent
    // would need `std::env::set_var`, which this crate's
    // `#![forbid(unsafe_code)]` rejects. The roundtrip test
    // `snapshot_dump_apply_preserves_anchor` proves dump+apply preserves the
    // anchor; the hook in `create_data_for_shielded_pool` is then a trivial
    // env-var read + delegation.
}
