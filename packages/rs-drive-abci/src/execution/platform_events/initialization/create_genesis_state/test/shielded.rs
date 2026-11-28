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
use drive::grovedb::engine::{DefaultEngine, Engine};
use drive::grovedb::Element;
use drive::grovedb::TransactionArg;
use drive::grovedb_path::SubtreePath;
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
        // precomputed snapshot through GroveDB's subtree load instead of running the (very
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
                combined_root = hex::encode(stats.combined_root),
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

            // Open a CommitmentTree on the storage engine's handle of the
            // subtree. The engine's batch buffers every BulkAppendTree /
            // frontier write and is applied to the transaction once at the
            // end — same shape as the production `commitment_tree_insert` path.
            let subtree_path_segs: Vec<Vec<u8>> = pool_path_arr
                .iter()
                .map(|s| s.to_vec())
                .chain(std::iter::once(leaf_key.to_vec()))
                .collect();
            let subtree_path_refs: Vec<&[u8]> =
                subtree_path_segs.iter().map(|v| v.as_slice()).collect();
            let subtree_path = SubtreePath::from(subtree_path_refs.as_slice());

            let engine = self.drive.grove.engine();
            let data_batch = <DefaultEngine as Engine>::Batch::default();
            let storage_ctx = engine.subtree(subtree_path, Some(&data_batch), tx).unwrap();
            let mut ct = CommitmentTree::<_, DashMemo>::open(
                0,
                chunk_power,
                storage_ctx,
                &platform_version.drive.grove_version,
            )
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
            // The whole `data_batch` is applied once at the end. A crash
            // mid-bake loses every batch; durability per batch would require
            // a batch applied and committed per round. For a bake binary
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
                ct.save(&platform_version.drive.grove_version)
                    .value
                    .map_err(|e| {
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
            ct.commit_mmr(&platform_version.drive.grove_version)
                .map_err(|e| {
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

            // Apply the subtree's data_batch to the transaction —
            // makes all the BulkAppendTree / frontier writes visible to
            // subsequent reads (including replace_subtree_root's own batch).
            engine.apply_batch(data_batch, tx).value.map_err(|e| {
                Error::Execution(ExecutionError::CorruptedCodeExecution(Box::leak(
                    format!("seed: apply the subtree's batch: {e}").into_boxed_str(),
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
        platform
            .drive
            .grove
            .commit_transaction(tx)
            .unwrap()
            .expect("commit");
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
        platform_a
            .drive
            .grove
            .commit_transaction(tx_a)
            .unwrap()
            .expect("commit A");
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
            "dump: total_count={} key_count={} body_bytes={}",
            dump_stats.total_count, dump_stats.key_count, dump_stats.body_bytes,
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

    /// An `InitChain` that applies the snapshot and never reaches the block-1
    /// commit (a later genesis/block-1 failure, a drive-abci restart, or a
    /// Tenderdash InitChain retry) must leave nothing behind, so the next
    /// attempt applies the snapshot to an empty pool again.
    ///
    /// The apply is a write of the transaction it is given: the loaded
    /// records and the parent-leaf patch leave with that transaction's
    /// rollback. (The first snapshot format ingested a file into the storage
    /// outside the transaction, and a retry then met the first attempt's
    /// records; this test reproduced the devnet that could never initialize.)
    ///
    /// This test models the scenario exactly:
    ///   1. apply the snapshot WITH a transaction (`Some(tx1)`);
    ///   2. DROP `tx1` without committing — the InitChain failure path. The
    ///      pool reads as empty again, through no transaction;
    ///   3. apply again WITH a fresh transaction (`Some(tx2)`) — the retry —
    ///      and commit it.
    #[test]
    fn snapshot_reapply_is_idempotent_under_initchain_retry() {
        let platform_version = PlatformVersion::latest();

        // --- A: seed a small pool and dump it to a snapshot file ---
        let (snapshot_dir, anchor_a) = dump_seeded_pool(5_000, platform_version);
        let snapshot_path = snapshot_dir.path().join("shielded-pool.snap");

        // --- B, attempt 1: apply inside a transaction, then ROLL BACK ---
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
            assert_eq!(
                read_current_anchor(&platform_b, Some(&tx1), platform_version),
                anchor_a,
                "the transaction that applied the snapshot reads the pool it loaded"
            );
            assert_eq!(
                read_current_anchor(&platform_b, None, platform_version),
                EMPTY_SINSEMILLA_ROOT,
                "nothing of the apply is committed before its transaction is"
            );
            // Drop tx1 WITHOUT committing — the failure/rollback path.
            drop(tx1);
        }
        assert_eq!(
            read_current_anchor(&platform_b, None, platform_version),
            EMPTY_SINSEMILLA_ROOT,
            "a rolled-back apply leaves the pool empty"
        );

        // --- B, attempt 2: the retry, in a fresh transaction ---
        let tx2 = platform_b.drive.grove.start_transaction();
        let retry = crate::shielded_snapshot::apply_shielded_snapshot(
            &platform_b.drive.grove,
            Some(&tx2),
            &snapshot_path,
            platform_version,
        );
        assert!(
            retry.is_ok(),
            "re-applying a genesis snapshot after a rolled-back attempt must \
             succeed (InitChain retry path), but it failed: {retry:?}"
        );
        platform_b
            .drive
            .grove
            .commit_transaction(tx2)
            .unwrap()
            .expect("commit retry tx");

        let anchor_b = read_current_anchor(&platform_b, None, platform_version);
        assert_eq!(anchor_b, anchor_a, "anchor must match after the retry");
    }

    /// Seed a pool of `total_notes`, dump it to `shielded-pool.snap` in a new
    /// temporary directory, and return the directory with the pool's anchor.
    fn dump_seeded_pool(
        total_notes: u32,
        platform_version: &PlatformVersion,
    ) -> (tempfile::TempDir, [u8; 32]) {
        let platform_a = build_regtest_platform();
        let cfg = ShieldedSeedConfig {
            total_notes,
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
        platform_a
            .drive
            .grove
            .commit_transaction(tx_a)
            .unwrap()
            .expect("commit A");
        let anchor_a = read_current_anchor(&platform_a, None, platform_version);
        assert_ne!(anchor_a, EMPTY_SINSEMILLA_ROOT);

        let dump_dir = tempfile::tempdir().expect("tempdir");
        crate::shielded_snapshot::dump_shielded_subtree(
            &platform_a.drive.grove,
            None,
            &dump_dir.path().join("shielded-pool.snap"),
            platform_version,
        )
        .expect("dump");
        (dump_dir, anchor_a)
    }

    /// The snapshot is only ever applied to an empty pool: a second apply,
    /// over the records the first one committed, is refused and changes
    /// nothing.
    #[test]
    fn snapshot_apply_refuses_a_populated_shielded_pool() {
        let platform_version = PlatformVersion::latest();
        let (snapshot_dir, anchor_a) = dump_seeded_pool(3_000, platform_version);
        let snapshot_path = snapshot_dir.path().join("shielded-pool.snap");

        let platform_b = build_regtest_platform();
        crate::shielded_snapshot::apply_shielded_snapshot(
            &platform_b.drive.grove,
            None,
            &snapshot_path,
            platform_version,
        )
        .expect("apply to the empty pool");
        assert_eq!(
            read_current_anchor(&platform_b, None, platform_version),
            anchor_a
        );

        let again = crate::shielded_snapshot::apply_shielded_snapshot(
            &platform_b.drive.grove,
            None,
            &snapshot_path,
            platform_version,
        );
        assert!(
            matches!(
                &again,
                Err(crate::shielded_snapshot::ShieldedSnapshotError::GroveDb(_))
            ),
            "a pool that holds records must refuse the snapshot: {again:?}"
        );
        assert_eq!(
            read_current_anchor(&platform_b, None, platform_version),
            anchor_a,
            "the refused apply changed nothing"
        );
    }

    /// Offsets of the snapshot header's fields: magic (8), format version (4),
    /// total count (8), chunk power (1), flags (2), combined root (32), body
    /// length (8).
    const HEADER_FORMAT_VERSION: std::ops::Range<usize> = 8..12;
    const HEADER_TOTAL_COUNT: std::ops::Range<usize> = 12..20;
    const HEADER_COMBINED_ROOT: std::ops::Range<usize> = 23..55;
    const HEADER_LEN: usize = 63;

    /// Write `bytes` as a snapshot file whose trailing checksum is right for
    /// them, so that a test reaches the checks behind the checksum.
    fn write_with_checksum(path: &std::path::Path, bytes: &[u8]) {
        let content = &bytes[..bytes.len() - 32];
        let mut out = content.to_vec();
        out.extend_from_slice(blake3::hash(content).as_bytes());
        std::fs::write(path, out).expect("write snapshot");
    }

    /// Each of these changes of a snapshot file is refused, and a refused
    /// apply with no transaction commits nothing: the version field alone set
    /// to 2, a changed byte of the body with the file's checksum as it was
    /// and made right, the file cut short at two lengths, bytes after the
    /// checksum, a header that states another root or another count than the
    /// dump, and a root that the header and the dump both state and the
    /// records do not give.
    #[test]
    fn snapshot_apply_rejects_a_damaged_file_and_commits_nothing() {
        use crate::shielded_snapshot::ShieldedSnapshotError;

        let platform_version = PlatformVersion::latest();
        let (snapshot_dir, _anchor_a) = dump_seeded_pool(3_000, platform_version);
        let good = std::fs::read(snapshot_dir.path().join("shielded-pool.snap")).expect("read");
        let damaged_path = snapshot_dir.path().join("damaged.snap");

        let platform_b = build_regtest_platform();
        let apply = |bytes: &[u8], checksum_made_right: bool| {
            if checksum_made_right {
                write_with_checksum(&damaged_path, bytes);
            } else {
                std::fs::write(&damaged_path, bytes).expect("write snapshot");
            }
            let result = crate::shielded_snapshot::apply_shielded_snapshot(
                &platform_b.drive.grove,
                None,
                &damaged_path,
                platform_version,
            );
            assert_eq!(
                read_current_anchor(&platform_b, None, platform_version),
                EMPTY_SINSEMILLA_ROOT,
                "a refused apply must commit nothing: {result:?}"
            );
            result.expect_err("a damaged snapshot must be refused")
        };

        // The version field of the format before this one, on a file that is
        // otherwise as written and whose checksum is made right: the version
        // is refused before the fields after it, the checksums and the load.
        let mut older = good.clone();
        older[HEADER_FORMAT_VERSION].copy_from_slice(&2u32.to_be_bytes());
        let error = apply(&older, true);
        assert!(
            matches!(
                error,
                ShieldedSnapshotError::FormatVersionMismatch {
                    expected: 3,
                    found: 2
                }
            ),
            "{error:?}"
        );

        // A byte of the body changed after the file was written.
        let mut flipped = good.clone();
        let last_byte_of_the_dump = good.len() - 32 - 1;
        flipped[last_byte_of_the_dump] ^= 0x01;
        let error = apply(&flipped, false);
        assert!(
            matches!(error, ShieldedSnapshotError::ChecksumMismatch { .. }),
            "{error:?}"
        );

        // The same change with the file's checksum made right: the dump's own
        // checksum refuses it.
        let error = apply(&flipped, true);
        assert!(
            matches!(error, ShieldedSnapshotError::Inconsistent(_)),
            "{error:?}"
        );

        // A file cut short.
        let error = apply(&good[..good.len() - 100], false);
        assert!(
            matches!(error, ShieldedSnapshotError::Inconsistent(_)),
            "{error:?}"
        );
        let error = apply(&good[..HEADER_LEN], false);
        assert!(
            matches!(error, ShieldedSnapshotError::Inconsistent(_)),
            "{error:?}"
        );

        // Bytes after the checksum.
        let mut longer = good.clone();
        longer.extend_from_slice(b"more");
        let error = apply(&longer, false);
        assert!(
            matches!(error, ShieldedSnapshotError::Inconsistent(_)),
            "{error:?}"
        );

        // A header that states another root than the dump.
        let mut other_root = good.clone();
        other_root[HEADER_COMBINED_ROOT.start] ^= 0x01;
        let error = apply(&other_root, true);
        assert!(
            matches!(error, ShieldedSnapshotError::Inconsistent(_)),
            "{error:?}"
        );

        // A root that the header and the dump both state and the records do
        // not give. The dump's checksum and the file's are made right, so the
        // load takes the records, and the roots read from them refuse it. The
        // dump states its root after its magic (8), its version (4), and the
        // element with its length (4).
        let mut wrong_root = other_root;
        let dump = HEADER_LEN..good.len() - 32;
        let element_len = u32::from_be_bytes(
            good[dump.start + 12..dump.start + 16]
                .try_into()
                .expect("four bytes"),
        ) as usize;
        let root_in_dump = dump.start + 16 + element_len;
        assert_eq!(
            good[root_in_dump..root_in_dump + 32],
            good[HEADER_COMBINED_ROOT],
            "the dump does not state its root there"
        );
        wrong_root[root_in_dump] ^= 0x01;
        let checksum_of_dump = dump.end - 32;
        let checksum = blake3::hash(&wrong_root[dump.start..checksum_of_dump]);
        wrong_root[checksum_of_dump..dump.end].copy_from_slice(checksum.as_bytes());
        let error = apply(&wrong_root, true);
        assert!(
            matches!(error, ShieldedSnapshotError::CombinedRootMismatch { .. }),
            "{error:?}"
        );

        // A header that states another leaf than the dump carries.
        let mut other_count = good.clone();
        other_count[HEADER_TOTAL_COUNT].copy_from_slice(&7u64.to_be_bytes());
        let error = apply(&other_count, true);
        assert!(
            matches!(error, ShieldedSnapshotError::Inconsistent(_)),
            "{error:?}"
        );

        // The file as it was written still applies after all of that.
        std::fs::write(&damaged_path, &good).expect("write snapshot");
        crate::shielded_snapshot::apply_shielded_snapshot(
            &platform_b.drive.grove,
            None,
            &damaged_path,
            platform_version,
        )
        .expect("the undamaged file applies");
    }

    // Real InitChain hook coverage happens via the dashmate devnet flow
    // (see docs/genesis-snapshot-design.md §13 e2e). An in-process equivalent
    // would need `std::env::set_var`, which this crate's
    // `#![forbid(unsafe_code)]` rejects. The roundtrip test
    // `snapshot_dump_apply_preserves_anchor` proves dump+apply preserves the
    // anchor; the hook in `create_data_for_shielded_pool` is then a trivial
    // env-var read + delegation.
}
