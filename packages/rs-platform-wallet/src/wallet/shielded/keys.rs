//! Orchard key management for the shielded wallet.
//!
//! Provides [`OrchardKeySet`] which derives the full ZIP-32 key hierarchy
//! from a wallet seed. The derivation path follows the Zcash Orchard spec:
//!
//!   `m / 32' / coin_type' / account'`
//!
//! where `coin_type` is 5 for Dash mainnet and 1 for testnet (BIP-44).
//!
//! All key types are re-exported from `grovedb_commitment_tree` which
//! wraps the upstream `orchard` crate.

use dashcore::Network;
use grovedb_commitment_tree::{
    FullViewingKey, IncomingViewingKey, OutgoingViewingKey, PaymentAddress,
    PreparedIncomingViewingKey, Scope, SpendAuthorizingKey, SpendingKey,
};
use zip32::AccountId;

use crate::error::PlatformWalletError;

/// Dash coin types per BIP-44.
const DASH_COIN_TYPE_MAINNET: u32 = 5;
const DASH_COIN_TYPE_TESTNET: u32 = 1;

/// Scrub-on-drop containment for an Orchard SECRET that provides no
/// `Zeroize` support — orchard 0.14's [`SpendingKey`] and
/// [`SpendAuthorizingKey`] are `Copy` types with neither a `Zeroize` impl
/// nor a scrubbing `Drop`, so a plain local holding one leaves the complete
/// spend-authority representation in its stack frame after use (#4204
/// review finding 1ee08ba70627).
///
/// The guard owns the value (`Deref` for use) and volatile-overwrites its
/// raw bytes on drop, then fences, so the scrub is not elided as a dead
/// store and runs on EVERY exit path (`?`, early return, panic-unwind).
/// Call sites additionally `drop()` the guard right after the secret's
/// final use so it never survives into long-lived async frames across
/// network awaits.
///
/// The safety argument is the `needs_drop` gate below: scrubbing is only
/// performed for types with no drop glue (both Orchard key types qualify —
/// `SpendingKey` is `Copy`; `SpendAuthorizingKey` is a plain scalar wrapper
/// with no `Drop`), so overwriting the bytes in place cannot double-free or
/// corrupt owned indirections. A type WITH drop glue is left untouched
/// (its own `Drop` still runs normally) — that would be a silent no-scrub,
/// so the guard is only for the two key types named above. (What no bound
/// can rule out is the caller having made further copies — the guard
/// contains the representation it owns; avoiding stray copies is the call
/// site's job.)
pub(crate) struct ScrubOnDrop<T>(pub(crate) T);

impl<T> Drop for ScrubOnDrop<T> {
    fn drop(&mut self) {
        // Const-folded: for the Orchard key types this is `false` and the
        // scrub always runs. Overwriting a value that still has drop glue
        // to execute would be unsound — skip (see the type-level docs).
        if core::mem::needs_drop::<T>() {
            return;
        }
        let ptr = &mut self.0 as *mut T as *mut u8;
        for i in 0..core::mem::size_of::<T>() {
            // Volatile per-byte overwrite: not removable as a dead store.
            unsafe { core::ptr::write_volatile(ptr.add(i), 0) };
        }
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }
}

impl<T> core::ops::Deref for ScrubOnDrop<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

#[cfg(test)]
mod scrub_tests {
    use super::*;

    /// Both Orchard secret types must stay scrubbable: drop glue appearing on
    /// either (an orchard upgrade adding `Drop`) would silently disable the
    /// scrub, and this is the tripwire that turns that into a test failure.
    #[test]
    fn orchard_secret_types_have_no_drop_glue() {
        assert!(!core::mem::needs_drop::<SpendingKey>());
        assert!(!core::mem::needs_drop::<SpendAuthorizingKey>());
    }
}

/// ZIP-32 derived Orchard key hierarchy.
///
/// Contains the key material needed for shielded sync and address
/// generation. The master `SpendingKey` is intentionally not retained:
/// it is derived inside [`Self::from_seed`] only long enough to extract
/// the FVK / ASK / IVK / OVK and is dropped before this struct is
/// returned. Spend authorization for an actual transaction re-derives
/// the SK transiently from the wallet seed via the host signer.
///
/// - `full_viewing_key` — derived from SK, can view all transactions
/// - `spend_auth_key` — signs individual spend authorizations
/// - `incoming_viewing_key` — detects incoming notes (trial decryption)
/// - `outgoing_viewing_key` — recovers sent notes (wallet recovery)
/// - `default_address` — the default payment address at index 0
pub struct OrchardKeySet {
    /// Full viewing key derived from the spending key.
    pub full_viewing_key: FullViewingKey,
    /// Spend authorization key for signing spends. Crate-private.
    pub(crate) spend_auth_key: SpendAuthorizingKey,
    /// Incoming viewing key for trial decryption.
    pub incoming_viewing_key: IncomingViewingKey,
    /// Outgoing viewing key for wallet recovery.
    pub outgoing_viewing_key: OutgoingViewingKey,
    /// Default payment address (index 0, external scope).
    pub default_address: PaymentAddress,
}

impl OrchardKeySet {
    /// Derive the full Orchard key set from a wallet seed.
    ///
    /// The `seed` should be the BIP-39 seed bytes (typically 64 bytes).
    /// ZIP-32 requires master seeds of 32-252 bytes; the underlying
    /// `SpendingKey::from_zip32_seed` does not enforce that bound
    /// itself, so it is checked here.
    ///
    /// # Errors
    ///
    /// Returns an error if the seed length is out of range or the
    /// ZIP-32 derivation fails (e.g. the derived key is the zero
    /// scalar).
    pub fn from_seed(
        seed: &[u8],
        network: Network,
        account: u32,
    ) -> Result<Self, PlatformWalletError> {
        if seed.len() < 32 || seed.len() > 252 {
            return Err(PlatformWalletError::ShieldedKeyDerivation(format!(
                "seed must be 32..=252 bytes per ZIP-32, got {}",
                seed.len()
            )));
        }

        let coin_type = match network {
            Network::Mainnet => DASH_COIN_TYPE_MAINNET,
            _ => DASH_COIN_TYPE_TESTNET,
        };

        let account_id = AccountId::try_from(account).map_err(|_| {
            PlatformWalletError::ShieldedKeyDerivation(format!(
                "invalid account index: {}",
                account
            ))
        })?;

        let sk = ScrubOnDrop(
            SpendingKey::from_zip32_seed(seed, coin_type, account_id).map_err(|e| {
                PlatformWalletError::ShieldedKeyDerivation(format!(
                    "ZIP-32 derivation failed: {}",
                    e
                ))
            })?,
        );

        let fvk = FullViewingKey::from(&*sk);
        let ask = SpendAuthorizingKey::from(&*sk);
        let ivk = fvk.to_ivk(Scope::External);
        let ovk = fvk.to_ovk(Scope::External);
        let default_address = fvk.address_at(0u32, Scope::External);
        // The master spending key's final use is behind us: scrub its bytes
        // NOW (the [`ScrubOnDrop`] guard volatile-zeroes them) rather than
        // letting the representation ride the rest of this frame. The
        // FVK / ASK / IVK / OVK already capture every quantity the wallet
        // needs; spend authorization is re-derived transiently from the
        // wallet seed via the host signer at sign time.
        drop(sk);

        Ok(Self {
            full_viewing_key: fvk,
            spend_auth_key: ask,
            incoming_viewing_key: ivk,
            outgoing_viewing_key: ovk,
            default_address,
        })
    }

    /// Derive a payment address at the given diversifier index.
    pub fn address_at(&self, index: u32) -> PaymentAddress {
        self.full_viewing_key.address_at(index, Scope::External)
    }

    /// Prepare the incoming viewing key for efficient trial decryption.
    ///
    /// `PreparedIncomingViewingKey` pre-computes values that are reused
    /// across many trial decryption attempts, making batch scanning faster.
    pub fn prepared_ivk(&self) -> PreparedIncomingViewingKey {
        PreparedIncomingViewingKey::new(&self.incoming_viewing_key)
    }

    /// Strip the spend-authorizing key and return only the
    /// viewing-grade material. Used to populate the
    /// network-scoped shielded coordinator's account registry —
    /// the coordinator runs sync (trial-decrypt + tree append +
    /// nullifier scan), none of which needs spend authority, so
    /// keeping the ASK on the per-wallet side preserves the
    /// privilege separation. Spend operations re-attach the ASK
    /// by passing the full [`OrchardKeySet`] back into the
    /// coordinator's spend methods at call time.
    pub fn viewing_keys(&self) -> AccountViewingKeys {
        AccountViewingKeys {
            full_viewing_key: self.full_viewing_key.clone(),
            incoming_viewing_key: self.incoming_viewing_key.clone(),
            prepared_ivk: self.prepared_ivk(),
            outgoing_viewing_key: self.outgoing_viewing_key.clone(),
            default_address: self.default_address,
        }
    }
}

/// Viewing-grade subset of an [`OrchardKeySet`] — the material
/// needed to detect, decrypt, and recover Orchard notes, with no
/// ability to authorize a spend.
///
/// The network-scoped shielded coordinator holds these for every
/// bound `(walletId, accountIndex)`; it never sees a
/// `SpendAuthorizingKey`. Spend operations re-derive the full
/// [`OrchardKeySet`] (ASK included) from the wallet seed for the
/// duration of that call only.
///
/// The whole struct is a pure function of the 96-byte raw FVK
/// encoding ([`Self::to_fvk_bytes`] / [`Self::from_fvk_bytes`]),
/// which is what hosts persist so a later launch can rebind the
/// shielded sub-wallet without touching the wallet seed.
#[derive(Clone)]
pub struct AccountViewingKeys {
    pub full_viewing_key: FullViewingKey,
    pub incoming_viewing_key: IncomingViewingKey,
    /// Pre-computed for fast trial-decrypt across many notes per
    /// sync pass. Cached at registration time so the sync loop
    /// doesn't pay [`PreparedIncomingViewingKey::new`] per pass.
    pub prepared_ivk: PreparedIncomingViewingKey,
    pub outgoing_viewing_key: OutgoingViewingKey,
    pub default_address: PaymentAddress,
}

impl AccountViewingKeys {
    /// Derive the full viewing-grade set from an Orchard
    /// `FullViewingKey`. IVK / OVK / default address are all pure
    /// functions of the FVK (external scope, diversifier index 0 —
    /// the same choices [`OrchardKeySet::from_seed`] makes), so a
    /// persisted FVK alone reconstructs everything sync needs.
    pub fn from_full_viewing_key(fvk: FullViewingKey) -> Self {
        let ivk = fvk.to_ivk(Scope::External);
        let ovk = fvk.to_ovk(Scope::External);
        let default_address = fvk.address_at(0u32, Scope::External);
        let prepared_ivk = PreparedIncomingViewingKey::new(&ivk);
        Self {
            full_viewing_key: fvk,
            incoming_viewing_key: ivk,
            prepared_ivk,
            outgoing_viewing_key: ovk,
            default_address,
        }
    }

    /// The raw 96-byte FVK encoding (`ak ‖ nk ‖ rivk`) — the only
    /// bytes a host has to persist to reconstruct this struct.
    pub fn to_fvk_bytes(&self) -> [u8; 96] {
        self.full_viewing_key.to_bytes()
    }

    /// Reconstruct from a persisted raw 96-byte FVK encoding.
    ///
    /// # Errors
    ///
    /// Returns an error when the bytes are not a canonical FVK
    /// encoding (any component fails its curve / field check).
    pub fn from_fvk_bytes(bytes: &[u8; 96]) -> Result<Self, PlatformWalletError> {
        let fvk = FullViewingKey::from_bytes(bytes).ok_or_else(|| {
            PlatformWalletError::ShieldedKeyDerivation(
                "persisted Orchard full viewing key bytes are not a canonical encoding".to_string(),
            )
        })?;
        Ok(Self::from_full_viewing_key(fvk))
    }
}

/// Length in bytes of a raw Orchard payment address: an 11-byte
/// diversifier concatenated with a 32-byte `pk_d`. This is the encoding
/// [`PaymentAddress::to_raw_address_bytes`] produces and the one
/// `platform_wallet_manager_shielded_default_address` /
/// `identity_create_from_one_time_key` speak.
pub const ORCHARD_RAW_ADDRESS_LEN: usize = 43;

/// Derive the default raw Orchard payment address (diversifier index 0,
/// external scope) from a 32-byte Orchard spending key.
///
/// This is the standalone, RNG-free deriver behind
/// [`generate_one_time_orchard_key`]. It runs the exact SK → FVK →
/// default-address pipeline that [`OrchardKeySet::from_seed`] uses
/// (`FullViewingKey::from(&sk)` then `address_at(0, External)`), and
/// returns the same 43-byte raw encoding
/// (`super::operations::identity_create_from_one_time_key` derives its
/// scan key from `SpendingKey::from_bytes(sk)` identically). The *inviter*
/// side of an L2 shielded invitation calls this to compute the Orchard
/// recipient it must fund a note to for a given one-time spending key; it
/// is also the cheap round-trip check for [`generate_one_time_orchard_key`].
///
/// # Errors
///
/// Returns [`PlatformWalletError::ShieldedKeyDerivation`] when `sk_bytes`
/// is not a valid Orchard `SpendingKey` scalar — the same validity gate
/// `identity_create_from_one_time_key` applies to a claimed key.
pub fn orchard_address_from_spending_key(
    sk_bytes: &[u8; 32],
) -> Result<[u8; ORCHARD_RAW_ADDRESS_LEN], PlatformWalletError> {
    // By-reference parameter: the caller's (typically `Zeroizing`) buffer is
    // not repeated as a plain by-value array at this boundary. The one
    // unavoidable transient copy is the `from_bytes` argument itself
    // (orchard's API takes the array by value); the RESULT is contained in a
    // [`ScrubOnDrop`] guard so the non-zeroizing `SpendingKey` representation
    // is volatile-scrubbed on every exit path (#4204 finding 1ee08ba70627).
    let sk = ScrubOnDrop(
        Option::<SpendingKey>::from(SpendingKey::from_bytes(*sk_bytes)).ok_or_else(|| {
            PlatformWalletError::ShieldedKeyDerivation(
                "spending key is not a valid Orchard SpendingKey".to_string(),
            )
        })?,
    );
    let fvk = FullViewingKey::from(&*sk);
    Ok(fvk.address_at(0u32, Scope::External).to_raw_address_bytes())
}

/// Generate a fresh one-time Orchard spending key together with its default
/// raw payment address.
///
/// Returns `(spending_key_32, default_address_43)`:
/// - `spending_key_32` — a uniformly random, valid 32-byte Orchard
///   `SpendingKey` scalar, wrapped in [`zeroize::Zeroizing`] so the bearer
///   secret is scrubbed when the caller drops it. These are exactly the bytes
///   `identity_create_from_one_time_key` accepts as its one-time key: both
///   sides round-trip through `SpendingKey::from_bytes`, which stores the
///   scalar bytes verbatim, so `spending_key_32 == sk.to_bytes()`.
/// - `default_address_43` — the address
///   [`orchard_address_from_spending_key`] derives for that key (raw
///   11-byte diversifier ‖ 32-byte `pk_d`).
///
/// This keeps all Orchard key material in Rust: the *inviter* funds a note
/// to `default_address_43`, and a *claimer* handed `spending_key_32`
/// re-derives the viewing keys and spends it.
///
/// The scalar is drawn from the OS CSPRNG ([`OsRng`](rand::rngs::OsRng))
/// and re-rolled until it is a valid Orchard key — an invalid draw is
/// negligibly rare and the same acceptance loop the `orchard` crate's own
/// dummy-key generator runs.
///
/// Uses [`RngCore::try_fill_bytes`] rather than `fill_bytes`: the latter
/// *panics* when the OS entropy source fails. This function is called from a
/// `#[no_mangle] extern "C"` FFI export, where a panic cannot unwind across
/// the C ABI and would abort the whole process before the JNI panic guard can
/// run. Surfacing the entropy failure as a typed
/// [`PlatformWalletError::ShieldedKeyDerivation`] instead lets the FFI layer
/// return a normal error to the host.
pub fn generate_one_time_orchard_key(
) -> Result<(zeroize::Zeroizing<[u8; 32]>, [u8; ORCHARD_RAW_ADDRESS_LEN]), PlatformWalletError> {
    use rand::{rngs::OsRng, RngCore};

    let mut rng = OsRng;
    loop {
        // `Zeroizing` inside the loop, not just on the accepted draw: the
        // acceptance loop can REJECT a draw, and a rejected 32-byte scalar is
        // still fresh CSPRNG key material. A plain `[u8; 32]` would drop at the
        // end of the iteration unscrubbed, leaving discarded near-keys in the
        // stack frame. Wrapping here scrubs every draw — rejected and accepted
        // alike — and carries the accepted one out to the caller still wrapped.
        let mut sk_bytes = zeroize::Zeroizing::new([0u8; 32]);
        rng.try_fill_bytes(sk_bytes.as_mut_slice()).map_err(|e| {
            PlatformWalletError::ShieldedKeyDerivation(format!(
                "OS RNG entropy source failed while generating a one-time Orchard key: {e}"
            ))
        })?;
        if let Some(sk) = Option::<SpendingKey>::from(SpendingKey::from_bytes(*sk_bytes)) {
            // Contain the accepted draw's non-zeroizing `SpendingKey`
            // representation too — the byte buffer is already `Zeroizing`,
            // but this derived form would otherwise die unscrubbed
            // (#4204 finding 1ee08ba70627).
            let sk = ScrubOnDrop(sk);
            let fvk = FullViewingKey::from(&*sk);
            let address = fvk.address_at(0u32, Scope::External).to_raw_address_bytes();
            return Ok((sk_bytes, address));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grovedb_commitment_tree::SpendingKey;

    // Test vector 0 from zcash-test-vectors
    // (`orchard_key_components.py`), the same data the orchard
    // fork's own `keys::tests::test_vectors` checks against. Here
    // it runs through the `grovedb_commitment_tree` re-exports —
    // the exact dependency chain `OrchardKeySet::from_seed` uses —
    // so a grovedb or orchard bump that changes sk → FVK / IVK /
    // OVK / address derivation fails this test, not a wallet sync
    // in the field.
    const TV_SK: [u8; 32] = [
        0x5d, 0x7a, 0x8f, 0x73, 0x9a, 0x2d, 0x9e, 0x94, 0x5b, 0x0c, 0xe1, 0x52, 0xa8, 0x04, 0x9e,
        0x29, 0x4c, 0x4d, 0x6e, 0x66, 0xb1, 0x64, 0x93, 0x9d, 0xaf, 0xfa, 0x2e, 0xf6, 0xee, 0x69,
        0x21, 0x48,
    ];
    const TV_IVK: [u8; 32] = [
        0x85, 0xc8, 0xb5, 0xcd, 0x1a, 0xc3, 0xec, 0x3a, 0xd7, 0x09, 0x21, 0x32, 0xf9, 0x7f, 0x01,
        0x78, 0xb0, 0x75, 0xc8, 0x1a, 0x13, 0x9f, 0xd4, 0x60, 0xbb, 0xe0, 0xdf, 0xcd, 0x75, 0x51,
        0x47, 0x24,
    ];
    const TV_OVK: [u8; 32] = [
        0xbc, 0xc7, 0x06, 0x5e, 0x59, 0x91, 0x0b, 0x35, 0x99, 0x3f, 0x59, 0x50, 0x5b, 0xe2, 0x09,
        0xb1, 0x4b, 0xf0, 0x24, 0x88, 0x75, 0x0b, 0xbc, 0x8b, 0x1a, 0xcd, 0xcf, 0x10, 0x8c, 0x36,
        0x20, 0x04,
    ];
    const TV_DK: [u8; 32] = [
        0x31, 0xd6, 0xa6, 0x85, 0xbe, 0x57, 0x0f, 0x9f, 0xaf, 0x3c, 0xa8, 0xb0, 0x52, 0xe8, 0x87,
        0x84, 0x0b, 0x2c, 0x9f, 0x8d, 0x67, 0x22, 0x4c, 0xa8, 0x2a, 0xef, 0xb9, 0xe2, 0xee, 0x5b,
        0xed, 0xaf,
    ];
    const TV_DEFAULT_D: [u8; 11] = [
        0x8f, 0xf3, 0x38, 0x69, 0x71, 0xcb, 0x64, 0xb8, 0xe7, 0x78, 0x99,
    ];
    const TV_DEFAULT_PK_D: [u8; 32] = [
        0x08, 0xdd, 0x8e, 0xbd, 0x7d, 0xe9, 0x2a, 0x68, 0xe5, 0x86, 0xa3, 0x4d, 0xb8, 0xfe, 0xa9,
        0x99, 0xef, 0xd2, 0x01, 0x6f, 0xae, 0x76, 0x75, 0x0a, 0xfa, 0xe7, 0xee, 0x94, 0x16, 0x46,
        0xbc, 0xb9,
    ];

    #[test]
    fn key_pipeline_matches_official_orchard_test_vector() {
        let sk = Option::<SpendingKey>::from(SpendingKey::from_bytes(TV_SK))
            .expect("test vector spending key is valid");
        let fvk = FullViewingKey::from(&sk);

        // Raw IVK encoding is dk ‖ ivk (Zcash protocol § 5.6.4.3).
        let ivk_bytes = fvk.to_ivk(Scope::External).to_bytes();
        assert_eq!(&ivk_bytes[..32], &TV_DK, "diversifier key mismatch");
        assert_eq!(&ivk_bytes[32..], &TV_IVK, "incoming viewing key mismatch");

        let ovk = fvk.to_ovk(Scope::External);
        assert_eq!(ovk.as_ref(), &TV_OVK, "outgoing viewing key mismatch");

        // Raw address encoding is d ‖ pk_d; the "default" address in
        // the vectors is diversifier index 0, which is what
        // `OrchardKeySet::from_seed` exposes as `default_address`.
        let raw = fvk.address_at(0u32, Scope::External).to_raw_address_bytes();
        assert_eq!(&raw[..11], &TV_DEFAULT_D, "default diversifier mismatch");
        assert_eq!(&raw[11..], &TV_DEFAULT_PK_D, "default pk_d mismatch");
    }

    #[test]
    fn from_seed_is_deterministic_and_domain_separated() {
        let seed = [0x42u8; 64];

        let a = OrchardKeySet::from_seed(&seed, Network::Testnet, 0).expect("derivation succeeds");
        let b = OrchardKeySet::from_seed(&seed, Network::Testnet, 0).expect("derivation succeeds");
        assert_eq!(
            a.default_address.to_raw_address_bytes(),
            b.default_address.to_raw_address_bytes(),
            "same seed/network/account must derive the same address"
        );
        assert_eq!(
            a.incoming_viewing_key.to_bytes(),
            b.incoming_viewing_key.to_bytes(),
            "same seed/network/account must derive the same IVK"
        );

        // coin_type 5 vs 1 — a mainnet wallet must not share keys
        // with a testnet wallet on the same seed.
        let mainnet =
            OrchardKeySet::from_seed(&seed, Network::Mainnet, 0).expect("derivation succeeds");
        assert_ne!(
            mainnet.default_address.to_raw_address_bytes(),
            a.default_address.to_raw_address_bytes(),
            "mainnet and testnet must derive different addresses"
        );

        // Devnet intentionally shares the testnet coin type (the
        // `_ =>` arm in `from_seed`).
        let devnet =
            OrchardKeySet::from_seed(&seed, Network::Devnet, 0).expect("derivation succeeds");
        assert_eq!(
            devnet.default_address.to_raw_address_bytes(),
            a.default_address.to_raw_address_bytes(),
            "devnet shares the testnet coin type"
        );

        let account1 =
            OrchardKeySet::from_seed(&seed, Network::Testnet, 1).expect("derivation succeeds");
        assert_ne!(
            account1.default_address.to_raw_address_bytes(),
            a.default_address.to_raw_address_bytes(),
            "accounts must derive different addresses"
        );

        // ZIP-32 hardened child bounds: account indices are u31.
        assert!(
            OrchardKeySet::from_seed(&seed, Network::Testnet, 1 << 31).is_err(),
            "account index ≥ 2^31 must be rejected"
        );
        // ZIP-32 master seed must be 32..=252 bytes.
        assert!(
            OrchardKeySet::from_seed(&[0u8; 16], Network::Testnet, 0).is_err(),
            "16-byte seed must be rejected"
        );
    }

    /// Known-answer pin for the full `from_seed` path (ZIP-32
    /// m/32'/1'/0' on a fixed seed). The expected bytes were
    /// generated by this code at the verified dependency pin
    /// (dashpay/orchard `dashified-0.14.0`, whose ZIP-32 and key
    /// test vectors pass upstream's official suite). If this test
    /// ever fails, derivation changed — existing wallets would
    /// stop seeing their notes. Do not update the constants
    /// without a migration story.
    #[test]
    fn from_seed_known_answer_pin() {
        let seed = [0x42u8; 64];
        let ks = OrchardKeySet::from_seed(&seed, Network::Testnet, 0).expect("derivation succeeds");

        let addr = ks.default_address.to_raw_address_bytes();
        let ivk = ks.incoming_viewing_key.to_bytes();

        const EXPECTED_ADDRESS: &str =
            "ee9f8174f92a3f035570ecbfe969aeb46f5e2f64ad69f78d34316c47ea38c2f0085b5788bebf478ce736a8";
        const EXPECTED_IVK: &str =
            "fae18cbcf032c37f646b0e3f211bda62dc79535f5276abbf274f46ba1d28d571946102f72db50fd672aadddc8346c513221c82e3fbc0c62058a2effb9669f228";
        assert_eq!(
            hex::encode(addr),
            EXPECTED_ADDRESS,
            "default address drifted"
        );
        assert_eq!(hex::encode(ivk), EXPECTED_IVK, "raw IVK encoding drifted");
    }

    /// The persisted-FVK round trip must reconstruct the exact
    /// viewing-grade set the seed derived: a launch that rebinds
    /// from the 96 persisted bytes has to trial-decrypt the same
    /// notes, recover the same send history (OVK), and show the
    /// same default address as the original seed bind — with no
    /// seed in reach.
    #[test]
    fn viewing_keys_round_trip_through_fvk_bytes() {
        let seed = [0x42u8; 64];
        let ks = OrchardKeySet::from_seed(&seed, Network::Testnet, 0).expect("derivation succeeds");
        let original = ks.viewing_keys();

        let fvk_bytes = original.to_fvk_bytes();
        let restored =
            AccountViewingKeys::from_fvk_bytes(&fvk_bytes).expect("persisted FVK decodes");

        assert_eq!(
            restored.full_viewing_key.to_bytes(),
            original.full_viewing_key.to_bytes(),
            "FVK drifted through the byte round trip"
        );
        assert_eq!(
            restored.incoming_viewing_key.to_bytes(),
            original.incoming_viewing_key.to_bytes(),
            "IVK must re-derive identically from the persisted FVK"
        );
        assert_eq!(
            restored.outgoing_viewing_key.as_ref(),
            original.outgoing_viewing_key.as_ref(),
            "OVK must re-derive identically from the persisted FVK"
        );
        assert_eq!(
            restored.default_address.to_raw_address_bytes(),
            original.default_address.to_raw_address_bytes(),
            "default address must re-derive identically from the persisted FVK"
        );

        // Corrupt encodings are rejected, not silently accepted.
        assert!(
            AccountViewingKeys::from_fvk_bytes(&[0xFFu8; 96]).is_err(),
            "non-canonical FVK bytes must be rejected"
        );
    }

    /// Round-trip: a freshly generated one-time key's returned address is
    /// exactly what [`orchard_address_from_spending_key`] re-derives from the
    /// returned spending key. This is the invariant the inviter/claimer split
    /// relies on — the inviter funds the returned address; the claimer, given
    /// only the spending key, must re-derive the same recipient.
    #[test]
    fn one_time_key_generate_roundtrips_to_its_address() {
        let (sk, address) = generate_one_time_orchard_key().expect("OS RNG available");
        let rederived = orchard_address_from_spending_key(&sk)
            .expect("a freshly generated sk is a valid Orchard SpendingKey");
        assert_eq!(
            address, rederived,
            "generated address must equal the deriver's output for the same sk"
        );
    }

    /// Ownership: a real Orchard note sent to the generated address is
    /// recognized by the generated key's incoming viewing key (the claimer
    /// discovers it on scan) and its nullifier derives cleanly under that
    /// key's full viewing key (the claimer can spend it). Mirrors the
    /// note-shaping the foreign-key scan in `operations.rs` performs.
    #[test]
    fn generated_key_owns_a_note_sent_to_its_address() {
        use grovedb_commitment_tree::{
            ExtractedNoteCommitment, FullViewingKey, Note, NoteValue, RandomSeed, Rho, Scope,
            SpendingKey,
        };

        let (sk_bytes, address_bytes) = generate_one_time_orchard_key().expect("OS RNG available");

        // Re-derive exactly the viewing keys a claimer would hold.
        let sk: SpendingKey = Option::from(SpendingKey::from_bytes(*sk_bytes))
            .expect("generated sk is a valid Orchard SpendingKey");
        let fvk = FullViewingKey::from(&sk);
        let ivk = fvk.to_ivk(Scope::External);
        let recipient = fvk.address_at(0u32, Scope::External);

        // The generated raw address is precisely this recipient.
        assert_eq!(
            recipient.to_raw_address_bytes(),
            address_bytes,
            "the generated address is the key's default payment address"
        );

        // The claimer's IVK owns (recognizes) that address.
        assert!(
            ivk.diversifier_index(&recipient).is_some(),
            "the generated key's ivk must own the generated address"
        );

        // Build a real note to the address (canonical rho / rseed, exactly as
        // the foreign-key scan reconstructs one) and confirm it is well-formed
        // and spendable under the generated fvk: the nullifier derives without
        // panicking, which is the quantity the claimer's scan stamps.
        let rho = (1u16..=u16::MAX)
            .find_map(|n| {
                let mut b = [0u8; 32];
                b[0..2].copy_from_slice(&n.to_le_bytes());
                Rho::from_bytes(&b).into_option()
            })
            .expect("a canonical rho exists");
        let rseed = (1u16..=u16::MAX)
            .find_map(|m| {
                let mut b = [0u8; 32];
                b[2..4].copy_from_slice(&m.to_le_bytes());
                RandomSeed::from_bytes(b, &rho).into_option()
            })
            .expect("a canonical rseed exists");
        let note = Note::from_parts(recipient, NoteValue::from_raw(10_000_000_000), rho, rseed)
            .into_option()
            .expect("valid note parts");

        let _cmx = ExtractedNoteCommitment::from(note.commitment()).to_bytes();
        let _nullifier = note.nullifier(&fvk).to_bytes();
        assert_eq!(
            note.recipient().to_raw_address_bytes(),
            address_bytes,
            "the note's recipient is the generated address"
        );
    }

    /// Determinism: the deriver is a pure function of the spending key —
    /// same sk in, same address out — and it agrees with what the generator
    /// returned.
    #[test]
    fn address_from_spending_key_is_deterministic() {
        let (sk, address) = generate_one_time_orchard_key().expect("OS RNG available");
        let a = orchard_address_from_spending_key(&sk).expect("valid sk");
        let b = orchard_address_from_spending_key(&sk).expect("valid sk");
        assert_eq!(a, b, "same sk must derive the same address");
        assert_eq!(
            a, address,
            "the deriver agrees with the generator for the generated sk"
        );
    }

    /// Two generations draw distinct keys (the OS CSPRNG is not seeded to a
    /// fixed value). A collision here would be a catastrophic RNG failure.
    #[test]
    fn generate_produces_distinct_keys() {
        let (sk_a, addr_a) = generate_one_time_orchard_key().expect("OS RNG available");
        let (sk_b, addr_b) = generate_one_time_orchard_key().expect("OS RNG available");
        assert_ne!(*sk_a, *sk_b, "distinct draws must differ");
        assert_ne!(
            addr_a, addr_b,
            "distinct keys must derive distinct addresses"
        );
    }
}
