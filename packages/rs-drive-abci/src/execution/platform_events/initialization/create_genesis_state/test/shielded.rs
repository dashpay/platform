//! Deterministic note generator for the SDK genesis test-data seeder.
//!
//! Two tiers:
//! - **Filler**: random valid Pallas-base `cmx` + random 32-byte ρ + 216 random
//!   bytes of "ciphertext". The wallet's compact decryption short-circuits on
//!   the ρ-field check, which is the intended "filler is not decryptable"
//!   failure mode.
//! - **Owned**: real Orchard `Note::from_parts(test_wallet_addr, value, ρ,
//!   rseed)` encrypted via `OrchardNoteEncryption::<DashMemo>` with
//!   `ovk = None`. The wallet's IVK trial-decrypts it and recovers the exact
//!   `NoteValue` we set.
//!
//! Both tiers go through `ShieldedPoolOperationType::InsertNote` and end up in
//! the production `commitment_tree_insert_op` code path. All randomness comes
//! from a single seeded `StdRng` threaded through every loop — no `OsRng`, no
//! `thread_rng()`. This is what makes the GroveDB root hash byte-identical
//! across hosts for a fixed seed.

use std::collections::HashSet;

use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::util::batch::drive_op_batch::{DriveOperation, ShieldedPoolOperationType};
use grovedb_commitment_tree::{
    DashMemo, Domain, ExtractedNoteCommitment, Note, NoteValue, OrchardDomain, RandomSeed, Rho,
    merkle_hash_from_bytes,
};
use orchard::note_encryption::OrchardNoteEncryption;
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};

use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::platform_types::platform::Platform;
use super::shielded_test_wallets::{TestWallet, test_wallet_a, test_wallet_b};

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
/// The chain's `create_data_for_shielded_pool` uses [`Self::sdk_test_data`] —
/// a hardcoded const-equivalent — so every SDK_TEST_DATA devnet seeds the same
/// 500k-note pool regardless of operator env. Tests construct custom configs
/// directly to vary N for unit + integration coverage.
#[derive(Debug, Clone)]
pub struct ShieldedSeedConfig {
    /// Total notes to seed across both tiers.
    pub total_notes: u32,
    /// Aggregate owned-note count across both wallets. Split evenly via
    /// [`Self::split_owned_count`].
    pub owned_count: u32,
    /// Per-owned-note value in credits.
    pub owned_value: u64,
    /// RNG seed; identical seed ⇒ identical root hash.
    pub rng_seed: u64,
}

impl Default for ShieldedSeedConfig {
    fn default() -> Self {
        Self {
            total_notes: 0,
            owned_count: 0,
            owned_value: 100_000,
            rng_seed: 0xDEAD_BEEF,
        }
    }
}

impl ShieldedSeedConfig {
    /// The hardcoded SDK_TEST_DATA seed config used at every devnet genesis.
    ///
    /// `total_notes = 500_000` (filler + owned), `owned_count = 8` split 4/4
    /// across wallets A and B, `owned_value = 100_000` ⇒ each wallet's
    /// expected balance after sync = `4 × 100_000 = 400_000`. Seed
    /// `0xDEAD_BEEF` is fixed so the GroveDB root hash is byte-identical
    /// across hosts.
    pub const fn sdk_test_data() -> Self {
        Self {
            total_notes: 500_000,
            owned_count: 8,
            owned_value: 100_000,
            rng_seed: 0xDEAD_BEEF,
        }
    }

    /// `(count_for_a, count_for_b)`. Even split; odd remainder goes to A.
    pub fn split_owned_count(&self) -> (u32, u32) {
        let a = self.owned_count.div_ceil(2);
        let b = self.owned_count - a;
        (a, b)
    }
}

/// Per-wallet deterministic position tables for owned notes.
#[derive(Debug, Clone, Default)]
pub struct OwnedLayout {
    pub positions_a: Vec<u32>,
    pub positions_b: Vec<u32>,
}

impl OwnedLayout {
    /// Compute owned positions: even stride across `0..total_notes`, with an
    /// rng-seed-derived offset. Even-indexed slots go to A, odd to B (after
    /// the even split).
    pub fn compute(cfg: &ShieldedSeedConfig) -> Self {
        if cfg.owned_count == 0 || cfg.total_notes == 0 {
            return Self::default();
        }
        let (count_a, count_b) = cfg.split_owned_count();
        let stride = (cfg.total_notes / cfg.owned_count).max(1);
        let offset = (cfg.rng_seed % u64::from(stride)) as u32;

        let mut positions_a = Vec::with_capacity(count_a as usize);
        let mut positions_b = Vec::with_capacity(count_b as usize);
        for i in 0..cfg.owned_count {
            let pos = stride * i + offset;
            if pos >= cfg.total_notes {
                // Defensive: owned_count > total_notes shouldn't happen with
                // sane configs, but if it does, drop the overflow.
                break;
            }
            if i % 2 == 0 && (positions_a.len() as u32) < count_a {
                positions_a.push(pos);
            } else if (positions_b.len() as u32) < count_b {
                positions_b.push(pos);
            } else {
                positions_a.push(pos);
            }
        }
        Self {
            positions_a,
            positions_b,
        }
    }

    /// Which wallet owns the given position? `Some(0)` = A, `Some(1)` = B,
    /// `None` = filler. O(N) lookup per call but N is tiny (≤ owned_count).
    pub fn wallet_at(&self, position: u32) -> Option<usize> {
        if self.positions_a.iter().any(|&p| p == position) {
            Some(0)
        } else if self.positions_b.iter().any(|&p| p == position) {
            Some(1)
        } else {
            None
        }
    }
}

/// A single seeded note ready to be wrapped in
/// `ShieldedPoolOperationType::InsertNote`.
#[derive(Debug, Clone)]
pub struct SeededNote {
    pub cmx: [u8; 32],
    /// On-wire `nullifier` field — this is ρ, *not* the spend-time revealed
    /// nullifier. The SDK reconstructs `OrchardDomain::for_compact_action` from
    /// these bytes during trial-decryption.
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
/// the field check — the cheap "filler is not decryptable" path.
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

/// Build one owned note encrypted to `wallet.default_address` with `value`
/// credits. Tracks ρ uniqueness in `used_rhos` across both wallets.
///
/// `out_ciphertext` is zero-filled, not produced by
/// `encrypt_outgoing_plaintext`. Rationale: the SDK's compact decryption path
/// (`decrypt.rs::try_decrypt_note`) never reads past byte `32 + COMPACT_NOTE_SIZE
/// = 84`, so the trailing 132 bytes are opaque to the consumer. Going through
/// `encrypt_outgoing_plaintext` would also require constructing a
/// `ValueCommitment` (no `Default` impl) for no observable behaviour change.
/// This matches the `orchard::note_encryption::testing::fake_compact_action`
/// pattern which similarly produces no `out_ciphertext`.
fn generate_owned_note(
    rng: &mut StdRng,
    wallet: &TestWallet,
    value: u64,
    used_rhos: &mut HashSet<[u8; 32]>,
) -> SeededNote {
    // 1. Valid Pallas-base ρ, unique across all owned notes.
    let rho_bytes = loop {
        let bytes = sample_valid_pallas_base(rng);
        if used_rhos.insert(bytes) {
            break bytes;
        }
    };
    let rho = Rho::from_bytes(&rho_bytes)
        .into_option()
        .expect("rho_bytes is a valid Pallas element by construction");

    // 2. Valid RandomSeed. RandomSeed::from_bytes can reject; loop until accepted.
    let rseed = loop {
        let mut bytes = [0u8; 32];
        rng.fill_bytes(&mut bytes);
        let candidate = RandomSeed::from_bytes(bytes, &rho);
        if candidate.is_some().into() {
            break candidate.unwrap();
        }
    };

    // 3. Build the Note.
    let note = Note::from_parts(wallet.default_address, NoteValue::from_raw(value), rho, rseed)
        .into_option()
        .expect("Note::from_parts must succeed for valid (addr, value, rho, rseed)");

    let cmx_bytes = ExtractedNoteCommitment::from(note.commitment()).to_bytes();

    // 4. Encrypt note plaintext via OrchardNoteEncryption<DashMemo>.
    let encryptor = OrchardNoteEncryption::<DashMemo>::new(None, note, [0u8; 36]);
    let epk_bytes = OrchardDomain::<DashMemo>::epk_bytes(encryptor.epk()).0;
    let enc_ciphertext = encryptor.encrypt_note_plaintext();

    // 5. Pack the 216-byte wire format. out_ciphertext = [0; 80]; see fn doc.
    let mut encrypted_note = Vec::with_capacity(ENCRYPTED_NOTE_WIRE_LEN);
    encrypted_note.extend_from_slice(&epk_bytes);
    encrypted_note.extend_from_slice(enc_ciphertext.as_ref());
    encrypted_note.extend_from_slice(&[0u8; 80]);
    debug_assert_eq!(encrypted_note.len(), ENCRYPTED_NOTE_WIRE_LEN);

    SeededNote {
        cmx: cmx_bytes,
        rho: rho_bytes,
        encrypted_note,
    }
}

/// Generate every seeded note in append order. Single seeded RNG threaded
/// through filler + owned tiers, so the output is byte-identical for a fixed
/// `cfg.rng_seed`.
pub fn generate_notes(cfg: &ShieldedSeedConfig, wallets: [&TestWallet; 2]) -> Vec<SeededNote> {
    let mut rng = StdRng::seed_from_u64(cfg.rng_seed);
    let layout = OwnedLayout::compute(cfg);
    let mut used_rhos: HashSet<[u8; 32]> = HashSet::with_capacity(cfg.owned_count as usize);
    let mut notes = Vec::with_capacity(cfg.total_notes as usize);

    for position in 0..cfg.total_notes {
        let note = match layout.wallet_at(position) {
            Some(idx) => generate_owned_note(&mut rng, wallets[idx], cfg.owned_value, &mut used_rhos),
            None => generate_filler_note(&mut rng),
        };
        notes.push(note);
    }

    debug_assert_eq!(notes.len(), cfg.total_notes as usize);
    notes
}

/// Convenience: resolve the two cached test wallets and generate notes.
pub fn generate_notes_for_test_wallets(cfg: &ShieldedSeedConfig) -> Vec<SeededNote> {
    generate_notes(cfg, [test_wallet_a(), test_wallet_b()])
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
        let cfg = ShieldedSeedConfig::sdk_test_data();
        tracing::info!(
            total_notes = cfg.total_notes,
            owned_count = cfg.owned_count,
            owned_value = cfg.owned_value,
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
            cfg_owned_count = cfg.owned_count,
            cfg_owned_value = cfg.owned_value,
            cfg_rng_seed = format!("0x{:x}", cfg.rng_seed),
            "seed_shielded_pool_with_config: entered"
        );
        if cfg.total_notes > 0 {
            tracing::info!(
                total_notes = cfg.total_notes,
                owned_count = cfg.owned_count,
                rng_seed = format!("0x{:x}", cfg.rng_seed),
                "seeding shielded pool with SDK test data"
            );

            // Generate every note up-front; single seeded RNG keeps the output
            // byte-identical across hosts. ρ uniqueness is enforced internally.
            let seeded = generate_notes_for_test_wallets(cfg);

            // One batched `apply_drive_operations` call. GroveDB's
            // `preprocess_commitment_tree_ops` groups every CommitmentTreeInsert
            // sharing this (path, key) into a single frontier-load /
            // append_with_mem_buffer loop / frontier-save / Merk propagation —
            // the amortization is structural, not aspirational.
            let operations: Vec<DriveOperation> = seeded
                .into_iter()
                .map(|n| {
                    DriveOperation::ShieldedPoolOperation(ShieldedPoolOperationType::InsertNote {
                        nullifier: n.rho,
                        cmx: n.cmx,
                        encrypted_note: n.encrypted_note,
                    })
                })
                .collect();

            self.drive.apply_drive_operations(
                operations,
                true,
                block_info,
                transaction,
                platform_version,
                None,
            )?;
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
            .record_shielded_pool_anchor_if_changed(
                GENESIS_ANCHOR_HEIGHT,
                tx,
                platform_version,
            )
            .map_err(Error::Drive)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grovedb_commitment_tree::{
        CompactAction, EphemeralKeyBytes, Nullifier, PaymentAddress, try_compact_note_decryption,
    };

    fn small_cfg() -> ShieldedSeedConfig {
        ShieldedSeedConfig {
            total_notes: 16,
            owned_count: 4,
            owned_value: 100_000,
            rng_seed: 0xDEAD_BEEF,
        }
    }

    #[test]
    fn split_owned_count_evenly_with_odd_remainder_to_a() {
        let cfg = ShieldedSeedConfig {
            owned_count: 7,
            ..ShieldedSeedConfig::default()
        };
        assert_eq!(cfg.split_owned_count(), (4, 3));
        let cfg = ShieldedSeedConfig {
            owned_count: 8,
            ..ShieldedSeedConfig::default()
        };
        assert_eq!(cfg.split_owned_count(), (4, 4));
        let cfg = ShieldedSeedConfig {
            owned_count: 0,
            ..ShieldedSeedConfig::default()
        };
        assert_eq!(cfg.split_owned_count(), (0, 0));
    }

    #[test]
    fn layout_assigns_positions_per_wallet_no_overlap() {
        let cfg = small_cfg();
        let layout = OwnedLayout::compute(&cfg);
        assert_eq!(layout.positions_a.len(), 2);
        assert_eq!(layout.positions_b.len(), 2);

        // No overlap.
        let mut all: Vec<u32> = layout
            .positions_a
            .iter()
            .chain(layout.positions_b.iter())
            .copied()
            .collect();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), 4);

        // All positions in range.
        assert!(all.iter().all(|&p| p < cfg.total_notes));
    }

    #[test]
    fn generate_notes_count_matches_total() {
        let cfg = small_cfg();
        let notes = generate_notes_for_test_wallets(&cfg);
        assert_eq!(notes.len(), cfg.total_notes as usize);
    }

    #[test]
    fn generate_notes_filler_ciphertext_size_pinned_to_216() {
        let cfg = small_cfg();
        let notes = generate_notes_for_test_wallets(&cfg);
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
        let a = generate_notes_for_test_wallets(&cfg);
        let b = generate_notes_for_test_wallets(&cfg);
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
        let a = generate_notes_for_test_wallets(&cfg_a);
        let b = generate_notes_for_test_wallets(&cfg_b);
        // At least one cmx differs (the first one — different RNG stream).
        assert!(a.iter().zip(b.iter()).any(|(na, nb)| na.cmx != nb.cmx));
    }

    #[test]
    fn owned_rhos_are_unique() {
        // ρ uniqueness is critical for Orchard correctness; protect future readers.
        let cfg = ShieldedSeedConfig {
            total_notes: 256,
            owned_count: 32,
            ..ShieldedSeedConfig::default()
        };
        let notes = generate_notes_for_test_wallets(&cfg);
        let layout = OwnedLayout::compute(&cfg);
        let mut owned_rhos: HashSet<[u8; 32]> = HashSet::new();
        for (pos, note) in notes.iter().enumerate() {
            if layout.wallet_at(pos as u32).is_some() {
                assert!(
                    owned_rhos.insert(note.rho),
                    "duplicate ρ at owned position {}",
                    pos
                );
            }
        }
        assert_eq!(owned_rhos.len(), cfg.owned_count as usize);
    }

    /// Wallet A's IVK must trial-decrypt every note at A's positions.
    /// This is the load-bearing test for the owned-tier encryption.
    #[test]
    fn owned_notes_decrypt_under_target_wallet_ivk() {
        let cfg = small_cfg();
        let layout = OwnedLayout::compute(&cfg);
        let notes = generate_notes_for_test_wallets(&cfg);
        let wallet_a = test_wallet_a();
        let wallet_b = test_wallet_b();

        // Wallet A's positions decrypt under A's IVK.
        for &pos in &layout.positions_a {
            let note = &notes[pos as usize];
            let decrypted = try_decrypt(note, &wallet_a.prepared_ivk);
            assert!(
                decrypted.is_some(),
                "wallet A should decrypt its own note at position {}",
                pos
            );
            let (recovered_note, _addr) = decrypted.unwrap();
            assert_eq!(recovered_note.value().inner(), cfg.owned_value);
        }

        // Wallet B's positions decrypt under B's IVK.
        for &pos in &layout.positions_b {
            let note = &notes[pos as usize];
            let decrypted = try_decrypt(note, &wallet_b.prepared_ivk);
            assert!(
                decrypted.is_some(),
                "wallet B should decrypt its own note at position {}",
                pos
            );
            let (recovered_note, _addr) = decrypted.unwrap();
            assert_eq!(recovered_note.value().inner(), cfg.owned_value);
        }
    }

    /// Cross-wallet privacy: A's IVK does not decrypt B's notes, and vice versa.
    /// This is the load-bearing test for §5.1's two-wallet rationale.
    #[test]
    fn cross_wallet_privacy_holds() {
        let cfg = small_cfg();
        let layout = OwnedLayout::compute(&cfg);
        let notes = generate_notes_for_test_wallets(&cfg);
        let wallet_a = test_wallet_a();
        let wallet_b = test_wallet_b();

        for &pos in &layout.positions_a {
            let note = &notes[pos as usize];
            assert!(
                try_decrypt(note, &wallet_b.prepared_ivk).is_none(),
                "wallet B must NOT decrypt wallet A's note at position {}",
                pos
            );
        }

        for &pos in &layout.positions_b {
            let note = &notes[pos as usize];
            assert!(
                try_decrypt(note, &wallet_a.prepared_ivk).is_none(),
                "wallet A must NOT decrypt wallet B's note at position {}",
                pos
            );
        }
    }

    /// The load-bearing test for the deterministic-balance claim. A real
    /// wallet does not know which positions are "owned" — it iterates the
    /// whole pool, trial-decrypts every note with its IVK, and sums the
    /// recovered `NoteValue`s. This test follows that exact pattern and
    /// asserts each wallet sees `count_per_wallet × owned_value` total,
    /// no false-positive decryptions, and no leakage across wallets.
    ///
    /// If this test ever fails, the seeded chain cannot meet the design
    /// doc's §9 acceptance criterion "wallet shows the expected balance"
    /// — i.e. the seeded notes cannot be safely shipped to consensus.
    #[test]
    fn each_wallet_sees_deterministic_aggregate_balance() {
        let cfg = small_cfg();
        let (count_a, count_b) = cfg.split_owned_count();
        let layout = OwnedLayout::compute(&cfg);
        let notes = generate_notes_for_test_wallets(&cfg);
        let wallet_a = test_wallet_a();
        let wallet_b = test_wallet_b();

        // Wallet A: walk the entire pool, sum recovered NoteValues from
        // successful trial-decryptions, count them, and cross-check each hit
        // against the expected owned position table.
        let mut a_balance: u64 = 0;
        let mut a_decrypts: u32 = 0;
        for (pos, note) in notes.iter().enumerate() {
            if let Some((recovered, _addr)) = try_decrypt(note, &wallet_a.prepared_ivk) {
                a_decrypts += 1;
                a_balance += recovered.value().inner();
                assert_eq!(
                    layout.wallet_at(pos as u32),
                    Some(0),
                    "wallet A decrypted at position {} which is not in its owned set",
                    pos
                );
            }
        }
        assert_eq!(a_decrypts, count_a, "wallet A decryption count mismatch");
        assert_eq!(
            a_balance,
            u64::from(count_a) * cfg.owned_value,
            "wallet A balance != count_a × owned_value"
        );

        // Wallet B: same scan.
        let mut b_balance: u64 = 0;
        let mut b_decrypts: u32 = 0;
        for (pos, note) in notes.iter().enumerate() {
            if let Some((recovered, _addr)) = try_decrypt(note, &wallet_b.prepared_ivk) {
                b_decrypts += 1;
                b_balance += recovered.value().inner();
                assert_eq!(
                    layout.wallet_at(pos as u32),
                    Some(1),
                    "wallet B decrypted at position {} which is not in its owned set",
                    pos
                );
            }
        }
        assert_eq!(b_decrypts, count_b, "wallet B decryption count mismatch");
        assert_eq!(
            b_balance,
            u64::from(count_b) * cfg.owned_value,
            "wallet B balance != count_b × owned_value"
        );

        // No overlap: an owned slot belongs to exactly one wallet.
        assert_eq!(
            a_decrypts + b_decrypts,
            cfg.owned_count,
            "owned-count invariant: A + B decryptions must sum to cfg.owned_count"
        );
    }

    /// Same as above, but exercises odd `owned_count` so A and B see different
    /// per-wallet counts. Pins the (count + 1)/2 split rule end-to-end.
    #[test]
    fn deterministic_balance_with_odd_owned_count_splits_correctly() {
        let cfg = ShieldedSeedConfig {
            total_notes: 32,
            owned_count: 7,
            owned_value: 50_000,
            rng_seed: 0xDEAD_BEEF,
        };
        let (count_a, count_b) = cfg.split_owned_count();
        assert_eq!((count_a, count_b), (4, 3));

        let notes = generate_notes_for_test_wallets(&cfg);
        let wallet_a = test_wallet_a();
        let wallet_b = test_wallet_b();

        let a_balance: u64 = notes
            .iter()
            .filter_map(|n| try_decrypt(n, &wallet_a.prepared_ivk))
            .map(|(note, _)| note.value().inner())
            .sum();
        let b_balance: u64 = notes
            .iter()
            .filter_map(|n| try_decrypt(n, &wallet_b.prepared_ivk))
            .map(|(note, _)| note.value().inner())
            .sum();

        assert_eq!(a_balance, u64::from(count_a) * cfg.owned_value); // 4 × 50_000 = 200_000
        assert_eq!(b_balance, u64::from(count_b) * cfg.owned_value); // 3 × 50_000 = 150_000
    }

    /// Filler notes are not decryptable by either wallet. ρ is random 32 bytes
    /// so `Nullifier::from_bytes` rejects roughly 50% of the time; the wallet
    /// returns `None` either way (rejected ρ or rejected plaintext).
    #[test]
    fn filler_notes_do_not_decrypt() {
        let cfg = small_cfg();
        let layout = OwnedLayout::compute(&cfg);
        let notes = generate_notes_for_test_wallets(&cfg);
        let wallet_a = test_wallet_a();
        let wallet_b = test_wallet_b();

        for (pos, note) in notes.iter().enumerate() {
            if layout.wallet_at(pos as u32).is_some() {
                continue;
            }
            assert!(try_decrypt(note, &wallet_a.prepared_ivk).is_none());
            assert!(try_decrypt(note, &wallet_b.prepared_ivk).is_none());
        }
    }

    /// Local trial-decrypt mirror of the SDK's `try_decrypt_note`. Lives here
    /// to avoid taking a dep on rs-sdk from rs-drive-abci.
    fn try_decrypt(
        note: &SeededNote,
        ivk: &grovedb_commitment_tree::PreparedIncomingViewingKey,
    ) -> Option<(Note, PaymentAddress)> {
        let nf = Nullifier::from_bytes(&note.rho).into_option()?;
        let cmx = ExtractedNoteCommitment::from_bytes(&note.cmx).into_option()?;
        let epk_bytes: [u8; 32] = note.encrypted_note[0..32].try_into().ok()?;
        let enc_compact: [u8; grovedb_commitment_tree::COMPACT_NOTE_SIZE] = note.encrypted_note
            [32..32 + grovedb_commitment_tree::COMPACT_NOTE_SIZE]
            .try_into()
            .ok()?;
        let compact = CompactAction::from_parts(nf, cmx, EphemeralKeyBytes(epk_bytes), enc_compact);
        let domain = OrchardDomain::<DashMemo>::for_compact_action(&compact);
        try_compact_note_decryption(&domain, ivk, &compact)
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
    use drive::drive::shielded::paths::{SHIELDED_NOTES_KEY, shielded_credit_pool_path};
    use grovedb_commitment_tree::EMPTY_SINSEMILLA_ROOT;

    /// Reduced default for integration tests — smaller is faster and still
    /// exercises every code path (filler, owned-A, owned-B, multi-chunk if N > 2048).
    fn integration_cfg() -> ShieldedSeedConfig {
        ShieldedSeedConfig {
            total_notes: 16,
            owned_count: 4,
            owned_value: 100_000,
            rng_seed: 0xDEAD_BEEF,
        }
    }

    /// `set_genesis_state` runs `create_sdk_test_data` under the cfg flag, which
    /// errors unless the platform is on the `Regtest` network. The default
    /// `TestPlatformBuilder::new()` config is mainnet, so every test in this
    /// module has to switch to regtest before calling `set_genesis_state`.
    fn build_regtest_platform()
    -> crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike> {
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
            .seed_shielded_pool_with_config(
                cfg,
                &BlockInfo::default(),
                Some(&tx),
                platform_version,
            )
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
            .seed_shielded_pool_with_config(&cfg, &BlockInfo::default(), Some(&tx), platform_version)
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
            owned_count: 0,
            ..ShieldedSeedConfig::default()
        };
        let anchor = build_and_seed(&cfg, platform_version);
        assert_eq!(anchor, EMPTY_SINSEMILLA_ROOT);
    }
}
