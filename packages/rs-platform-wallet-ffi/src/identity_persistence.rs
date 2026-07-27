//! FFI types + helpers for forwarding
//! [`IdentityChangeSet`](platform_wallet::IdentityChangeSet) and
//! [`IdentityKeysChangeSet`](platform_wallet::IdentityKeysChangeSet)
//! out of [`FFIPersister`](crate::persistence::FFIPersister) to Swift.
//!
//! The shape maps 1:1 onto Swift's `PersistentIdentity` +
//! `PersistentPublicKey` SwiftData models so the Swift handler can
//! apply each changeset as plain row upserts/removes.
//!
//! ## Ownership
//!
//! Both `IdentityEntryFFI` and `IdentityKeyEntryFFI` carry heap
//! allocations (C-strings for label / derivation path, byte buffers
//! for `public_key_data`). Each struct has a paired free helper that
//! releases every allocation it owns. The callback callers
//! ([`persistence.rs`](crate::persistence)) build temporary arrays of
//! these structs, fire the callback synchronously, then call the free
//! helper for every entry before returning — Swift must consume
//! whatever it needs to persist before returning from the callback.

use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

use platform_wallet::changeset::{IdentityEntry, IdentityKeyEntry};

// `IdentityStatus` discriminants are mirrored on the Swift side. Keep
// this encoding in sync with the `repr(u8)` order in
// `platform-wallet/src/wallet/identity/types/key_storage.rs`.
use platform_wallet::{DashPayProfile, IdentityStatus};

/// Flat C mirror of [`IdentityEntry`]'s persistable scalars.
///
/// Public keys are NOT included here — they travel in
/// [`IdentityKeyEntryFFI`] alongside their derivation breadcrumb via
/// a separate callback. Fields that don't map onto the Swift schema
/// (block times, contested DPNS names, DashPay payments) are skipped;
/// DashPay payment overlays already ride on the dedicated
/// `dashpay_payments_overlay` surface on the parent changeset.
///
/// User-visible label is no longer carried — `ManagedIdentity` doesn't
/// have one, and Swift owns the `PersistentIdentity.alias` column
/// directly. Removed entirely so the FFI layout stays minimal.
///
/// Settled DPNS labels DO ride on this struct (heap-allocated, freed
/// in [`free_identity_entry_ffi`]) so the Swift persister can
/// upsert/cascade them onto a `PersistentDPNSName` row collection
/// owned by the parent `PersistentIdentity`. Contested labels are
/// deliberately omitted — their lifecycle is in-flight contest churn,
/// not the settled-label collection this struct mirrors.
///
/// DashPay profile (`dashpay_profile_*`) rides on every upsert when
/// the underlying [`IdentityEntry::dashpay_profile`] is `Some(_)`. The
/// `_present` flag plus the per-string nullable pointers let Swift
/// distinguish "no profile yet" (skip the row) from "profile present
/// with this field unset" (clear the column). All heap-allocated
/// strings are freed in [`free_identity_entry_ffi`].
#[repr(C)]
pub struct IdentityEntryFFI {
    pub identity_id: [u8; 32],
    pub balance: u64,
    pub revision: u64,
    /// Set iff the underlying `IdentityEntry.identity_index` is
    /// `Some(_)`. Out-of-wallet identities have no derivation context
    /// and surface here with `identity_index_is_some == false`.
    pub identity_index_is_some: bool,
    /// BIP-9 HD identity index. Included so Swift can reconstruct the
    /// derivation path on watch-only restore. Ignore unless
    /// `identity_index_is_some` is `true`.
    pub identity_index: u32,
    /// `IdentityStatus` discriminant (see DPP enum).
    pub status: u8,
    /// Set iff the identity carries a wallet id. Swift uses this to
    /// link `PersistentIdentity.walletId` back to `PersistentWallet`.
    pub wallet_id_is_some: bool,
    pub wallet_id: [u8; 32],
    /// Heap-allocated array of NUL-terminated UTF-8 C strings, one
    /// per confirmed DPNS label on the underlying
    /// [`IdentityEntry::dpns_names`]. Owned by this FFI struct; freed
    /// in [`free_identity_entry_ffi`]. `null` when `dpns_names_count`
    /// is 0.
    ///
    /// Inner pointers may individually be null when the source label
    /// contained an interior NUL byte (unreachable in practice — DPNS
    /// validation rejects them). Consumers must skip null inner
    /// pointers.
    pub dpns_names: *const *const c_char,
    /// Number of entries pointed at by [`Self::dpns_names`] /
    /// [`Self::dpns_names_acquired_at`]. The two arrays are always the
    /// same length.
    pub dpns_names_count: usize,
    /// Parallel `u64` array of `acquired_at` Unix-millis timestamps;
    /// `0` when the source `DpnsNameInfo.acquired_at` was `None`.
    /// Same length as [`Self::dpns_names`]. Heap-allocated, freed in
    /// [`free_identity_entry_ffi`]. `null` when count is 0.
    pub dpns_names_acquired_at: *const u64,
    /// `true` iff the underlying [`IdentityEntry::dashpay_profile`]
    /// is `Some(_)`. When `false`, all `dashpay_profile_*` pointer
    /// fields are null and the byte-array fields are zeroed — Swift
    /// must skip the profile upsert entirely (changeset semantics:
    /// `dashpay_profile: None` means "no update" rather than
    /// "delete", matching the merge policy on the Rust side).
    pub dashpay_profile_present: bool,
    /// Heap-allocated NUL-terminated UTF-8 C string for the DashPay
    /// profile's display name. `null` when the source field was
    /// `None`. Owned by this FFI struct; freed in
    /// [`free_identity_entry_ffi`]. Ignore unless
    /// [`Self::dashpay_profile_present`] is `true`.
    pub dashpay_profile_display_name: *const c_char,
    /// Heap-allocated NUL-terminated UTF-8 C string for the DashPay
    /// profile's bio. `null` when the source field was `None`. Owned
    /// by this FFI struct; freed in [`free_identity_entry_ffi`].
    /// Ignore unless [`Self::dashpay_profile_present`] is `true`.
    pub dashpay_profile_bio: *const c_char,
    /// Heap-allocated NUL-terminated UTF-8 C string for the DashPay
    /// profile's avatar URL. `null` when the source field was
    /// `None`. Owned by this FFI struct; freed in
    /// [`free_identity_entry_ffi`]. Ignore unless
    /// [`Self::dashpay_profile_present`] is `true`.
    pub dashpay_profile_avatar_url: *const c_char,
    /// SHA-256 hash of the avatar image bytes (DIP-15 `avatarHash`).
    /// Zeroed when the source `Option<[u8; 32]>` was `None` — gate
    /// reads on [`Self::dashpay_profile_avatar_hash_present`] rather
    /// than checking for an all-zero hash, since `[0u8; 32]` is a
    /// valid (if cosmically unlikely) hash value.
    pub dashpay_profile_avatar_hash: [u8; 32],
    /// `true` iff the source `avatar_hash` was `Some(_)`. Disambiguates
    /// "no hash" from "hash that happens to be all zeros".
    pub dashpay_profile_avatar_hash_present: bool,
    /// DHash perceptual fingerprint of the avatar image (DIP-15
    /// `avatarFingerprint`, 8 bytes / 64 bits). Zeroed when the source
    /// `Option<[u8; 8]>` was `None` — gate reads on
    /// [`Self::dashpay_profile_avatar_fingerprint_present`] rather
    /// than checking for an all-zero fingerprint.
    pub dashpay_profile_avatar_fingerprint: [u8; 8],
    /// `true` iff the source `avatar_fingerprint` was `Some(_)`.
    pub dashpay_profile_avatar_fingerprint_present: bool,
    /// Heap-allocated NUL-terminated UTF-8 C string for the DashPay
    /// profile's public message. `null` when the source field was
    /// `None`. Owned by this FFI struct; freed in
    /// [`free_identity_entry_ffi`]. Ignore unless
    /// [`Self::dashpay_profile_present`] is `true`.
    pub dashpay_profile_public_message: *const c_char,
    /// Heap-allocated array of [`ContactProfileRowFFI`], one per entry
    /// of the underlying [`IdentityEntry::contact_profiles`] map —
    /// present profiles as full rows, confirmed-absent entries as
    /// `is_present == false` tombstone rows instructing the consumer to
    /// DELETE its persisted row (see [`ContactProfileRowFFI`]). Each row
    /// owns the same per-string heap allocations the own-profile block
    /// does; every string plus the outer boxed slice is released in
    /// [`free_identity_entry_ffi`]. `null` when
    /// [`Self::contact_profiles_count`] is 0.
    ///
    /// Distinct from `dashpay_profile_*` above: that block is the
    /// owner's *own* profile (one per identity); this array is the
    /// *contacts'* profiles, keyed by each contact's identity id. They
    /// land in separate SwiftData stores on the Swift side.
    pub contact_profiles: *const ContactProfileRowFFI,
    /// Number of rows pointed at by [`Self::contact_profiles`]. `0`
    /// when the identity has no present cached contact profiles.
    pub contact_profiles_count: usize,
}

/// Flat C mirror of one **present** cached contact profile —
/// `(contact_id, DashPayProfile, checked_at_ms)` — projected from a
/// single entry of [`IdentityEntry::contact_profiles`].
///
/// The profile-field block (`display_name` … `public_message`) is the
/// SAME shape as the own-profile fields on [`IdentityEntryFFI`]; the
/// only additions are the leading `contact_id` key, the `is_present`
/// discriminator, and the trailing `checked_at_ms` self-heal timestamp.
/// Confirmed-absent cache entries DO reach this struct, as
/// `is_present == false` tombstones (null strings / zeroed bytes) that
/// tell Swift to delete the persisted row.
///
/// All four `*const c_char` strings are heap-allocated via
/// [`optional_c_string`] and owned by the parent [`IdentityEntryFFI`];
/// they are released row-by-row in [`free_identity_entry_ffi`] before
/// the outer boxed slice drops. Gate the byte-array fields on their
/// paired `_present` flag — `[0u8; N]` is a valid (if unlikely) hash /
/// fingerprint value.
#[repr(C)]
pub struct ContactProfileRowFFI {
    /// The contact's 32-byte identity id — the
    /// [`IdentityEntry::contact_profiles`] map key. Becomes the
    /// `contactIdentityId` half of the SwiftData row's compound key.
    pub contact_id: [u8; 32],
    /// `true` for a present profile (all fields below are authoritative);
    /// `false` for a confirmed-absent contact (every field below is
    /// null/zeroed). An absent row tells Swift to DELETE any persisted row
    /// for `contact_id` — a contact who removed their profile must not keep
    /// showing a stale name/avatar. Without this, the persist side emits only
    /// present profiles and the Swift upsert never learns about a deletion.
    pub is_present: bool,
    /// Heap-allocated `displayName`; `null` when the source field was
    /// `None` (or the row is absent). Freed in [`free_identity_entry_ffi`].
    pub display_name: *const c_char,
    /// Heap-allocated `bio`; `null` when `None`. Freed in
    /// [`free_identity_entry_ffi`].
    pub bio: *const c_char,
    /// Heap-allocated `avatarUrl`; `null` when `None`. Freed in
    /// [`free_identity_entry_ffi`].
    pub avatar_url: *const c_char,
    /// SHA-256 avatar hash; zeroed when [`Self::avatar_hash_present`]
    /// is `false`.
    pub avatar_hash: [u8; 32],
    /// `true` iff the source `avatar_hash` was `Some(_)`.
    pub avatar_hash_present: bool,
    /// DHash avatar fingerprint; zeroed when
    /// [`Self::avatar_fingerprint_present`] is `false`.
    pub avatar_fingerprint: [u8; 8],
    /// `true` iff the source `avatar_fingerprint` was `Some(_)`.
    pub avatar_fingerprint_present: bool,
    /// Heap-allocated `publicMessage`; `null` when `None`. Freed in
    /// [`free_identity_entry_ffi`].
    pub public_message: *const c_char,
    /// Wall-clock ms of the last fetch attempt — the
    /// [`ContactProfileEntry::checked_at_ms`] self-heal timestamp.
    pub checked_at_ms: u64,
}

/// Flat C mirror of [`IdentityKeyEntry`] for forwarding across FFI.
///
/// No private material crosses here. For a key this wallet's seed reproduced
/// under discovery's verify gate, the client gets the
/// `(wallet_id, identity_index, key_index)` breadcrumb and derives the key on
/// demand from the Keychain seed at the DIP-9 path when it needs to sign; a
/// watch-only key carries no breadcrumb. `public_key_hash` is the precomputed
/// RIPEMD160(SHA256) of the pubkey so clients without a RIPEMD-160
/// implementation can still round-trip it into the keychain.
///
/// `public_key_data_ptr` / `public_key_data_len` own a heap-allocated
/// copy of the public-key bytes (compressed secp256k1 for ECDSA, hash
/// for hash160, etc. — depends on `key_type`). Released by
/// [`free_identity_key_entry_ffi`].
#[repr(C)]
pub struct IdentityKeyEntryFFI {
    pub identity_id: [u8; 32],
    pub key_id: u32,

    // IdentityPublicKey mirror — mirrors the layout of the existing
    // `IdentityPublicKeyFFI` so Swift can share decoder logic if it
    // wants to; we keep this struct self-contained to avoid cross-file
    // coupling on the Swift side.
    pub purpose: u8,
    pub security_level: u8,
    pub key_type: u8,
    pub read_only: bool,
    pub disabled_at_is_some: bool,
    pub disabled_at: u64,
    pub public_key_data_ptr: *mut u8,
    pub public_key_data_len: usize,
    /// 20-byte RIPEMD160(SHA256) of the public-key bytes.
    pub public_key_hash: [u8; 20],

    // Derivation breadcrumb. When both `wallet_id_is_some` and
    // `derivation_indices_is_some` are true the client should
    // re-derive the 32-byte ECDSA scalar from the named wallet's
    // mnemonic at the DIP-9 identity authentication path
    // `m/9'/coin'/5'/0'/ECDSA'/identity_index'/key_index'` and
    // persist it locally. When either flag is false the key is
    // watch-only.
    pub wallet_id_is_some: bool,
    pub wallet_id: [u8; 32],
    pub derivation_indices_is_some: bool,
    pub identity_index: u32,
    pub key_index: u32,

    // ContractBounds projection. Mirrors the DPP enum
    // `ContractBounds` so the client can reconstruct the variant
    // verbatim instead of dropping the document-type name:
    //
    //   * `contract_bounds_kind == 0` — no contract bounds; the
    //     `id` field is zeroed and the doc-type pointer is null.
    //   * `contract_bounds_kind == 1` — `SingleContract`; only the
    //     32-byte `id` is meaningful, doc-type pointer is null.
    //   * `contract_bounds_kind == 2` — `SingleContractDocumentType`;
    //     both the `id` and the heap-allocated UTF-8 doc-type
    //     C-string are meaningful. Doc-type string is released by
    //     [`free_identity_key_entry_ffi`].
    //
    // Keeping the kind tag inline (vs. always nulling fields) lets
    // the Swift side switch on a single discriminant without
    // probing pointer values, matching how the rest of this struct
    // encodes optional payloads.
    //
    // Ownership: `contract_bounds_document_type` is owned by this
    // struct EXCLUSIVELY when it is populated by
    // [`IdentityKeyEntryFFI::from_entry`]. Consumers MUST NOT
    // copy the struct value and then free both copies — the second
    // free is a use-after-free / double-free. The Swift binding
    // copies the doc-type into an owned Swift `String` inside the
    // callback (per `persistIdentityKeysCallback`) and never
    // retains the raw pointer past the callback window, which is
    // the only supported consumption pattern.
    pub contract_bounds_kind: u8,
    pub contract_bounds_id: [u8; 32],
    pub contract_bounds_document_type: *const c_char,
}

/// Composite identifier for [`IdentityKeysChangeSet::removed`] entries
/// on the FFI boundary. A flat `[u8; 32]` + `u32` pair so Swift can
/// iterate an array directly without a secondary indirection.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IdentityKeyRemovalFFI {
    pub identity_id: [u8; 32],
    pub key_id: u32,
}

// Compile-time guard — if anyone reshapes `IdentityKeyEntryFFI`, cargo
// builds fail with an obvious size mismatch here rather than producing a
// dylib the Swift side mis-parses at runtime (which surfaces as a random
// EXC_BAD_ACCESS in the persistIdentityKeys callback). There is no
// hand-maintained Swift mirror struct: cbindgen regenerates the C header
// from this definition at build time (`build.rs`), so Swift auto-sees the
// fields after the framework is rebuilt; only `persistIdentityKeysCallback`
// reads them.
//
// Expected layout on 64-bit targets (all fields in declaration
// order under `#[repr(C)]`):
//
//   0..=31    identity_id             [u8; 32]
//   32..=35   key_id                  u32
//   36        purpose                 u8
//   37        security_level          u8
//   38        key_type                u8
//   39        read_only               bool
//   40        disabled_at_is_some     bool
//   41..=47   (padding to 8)
//   48..=55   disabled_at             u64
//   56..=63   public_key_data_ptr     *mut u8
//   64..=71   public_key_data_len     usize
//   72..=91   public_key_hash         [u8; 20]
//   92        wallet_id_is_some       bool
//   93..=124  wallet_id               [u8; 32]
//   125       derivation_indices_is_some bool
//   126..=127 (padding to 4)
//   128..=131 identity_index          u32
//   132..=135 key_index               u32
//   136       contract_bounds_kind    u8
//   137..=168 contract_bounds_id      [u8; 32]
//   169..=175 (padding to 8 for pointer alignment)
//   176..=183 contract_bounds_document_type *const c_char
//
// Total size = 184, alignment = 8 (from u64 / pointer).
const _: [u8; 184] = [0u8; std::mem::size_of::<IdentityKeyEntryFFI>()];
const _: [u8; 8] = [0u8; std::mem::align_of::<IdentityKeyEntryFFI>()];

// Compile-time guard for `IdentityEntryFFI`. Same rationale as the
// `IdentityKeyEntryFFI` guard above — the Swift side picks up the
// header layout via cbindgen, so a layout drift would manifest as a
// random `EXC_BAD_ACCESS` in the persistIdentities callback rather
// than a build error. Pin the expected size here so any reshape
// fails the cargo build first.
//
// Expected layout on 64-bit targets (all fields in declaration
// order under `#[repr(C)]`):
//
//   0..=31    identity_id                              [u8; 32]
//   32..=39   balance                                  u64
//   40..=47   revision                                 u64
//   48        identity_index_is_some                   bool
//   49..=51   (padding to 4)
//   52..=55   identity_index                           u32
//   56        status                                   u8
//   57        wallet_id_is_some                        bool
//   58..=89   wallet_id                                [u8; 32]
//   90..=95   (padding to 8 for pointer alignment)
//   96..=103  dpns_names                               *const *const c_char
//   104..=111 dpns_names_count                         usize
//   112..=119 dpns_names_acquired_at                   *const u64
//   120       dashpay_profile_present                  bool
//   121..=127 (padding to 8 for pointer alignment)
//   128..=135 dashpay_profile_display_name             *const c_char
//   136..=143 dashpay_profile_bio                      *const c_char
//   144..=151 dashpay_profile_avatar_url               *const c_char
//   152..=183 dashpay_profile_avatar_hash              [u8; 32]
//   184       dashpay_profile_avatar_hash_present      bool
//   185..=192 dashpay_profile_avatar_fingerprint       [u8; 8]
//   193       dashpay_profile_avatar_fingerprint_present bool
//   194..=199 (padding to 8 for pointer alignment)
//   200..=207 dashpay_profile_public_message           *const c_char
//   208..=215 contact_profiles                         *const ContactProfileRowFFI
//   216..=223 contact_profiles_count                   usize
//
// Total size = 224, alignment = 8 (from u64 / pointer).
const _: [u8; 224] = [0u8; std::mem::size_of::<IdentityEntryFFI>()];
const _: [u8; 8] = [0u8; std::mem::align_of::<IdentityEntryFFI>()];

// ---------------------------------------------------------------------------
// Conversions
// ---------------------------------------------------------------------------

impl IdentityEntryFFI {
    /// Copy an [`IdentityEntry`] into a fresh FFI struct.
    ///
    /// Allocates two parallel heap arrays for the DPNS labels:
    /// `dpns_names` (a boxed slice of `CString::into_raw` pointers)
    /// and `dpns_names_acquired_at` (a boxed slice of timestamps).
    /// Both are released by [`free_identity_entry_ffi`] which the
    /// persister callsite calls after the Swift handler returns.
    ///
    /// When [`IdentityEntry::dashpay_profile`] is `Some(_)` the
    /// per-string profile fields are heap-allocated `CString`s
    /// (released by [`free_identity_entry_ffi`]) and the
    /// `_present` flag is set to `true`. When the profile is
    /// `None` every profile field is zero/null and the flag is
    /// `false`.
    pub fn from_entry(entry: &IdentityEntry) -> Self {
        let (wallet_id_is_some, wallet_id) = match entry.wallet_id {
            Some(id) => (true, id),
            None => (false, [0u8; 32]),
        };
        let (identity_index_is_some, identity_index) = match entry.identity_index {
            Some(idx) => (true, idx),
            None => (false, 0),
        };

        let (dpns_names, dpns_names_acquired_at, dpns_names_count) =
            allocate_dpns_arrays(&entry.dpns_names);

        let profile_fields = match &entry.dashpay_profile {
            Some(profile) => DashPayProfileFields::from_profile(profile),
            None => DashPayProfileFields::absent(),
        };

        let (contact_profiles, contact_profiles_count) =
            allocate_contact_profile_rows(&entry.contact_profiles);

        Self {
            identity_id: entry.id.to_buffer(),
            balance: entry.balance,
            revision: entry.revision,
            identity_index_is_some,
            identity_index,
            status: status_discriminant(entry.status),
            wallet_id_is_some,
            wallet_id,
            dpns_names,
            dpns_names_count,
            dpns_names_acquired_at,
            dashpay_profile_present: profile_fields.present,
            dashpay_profile_display_name: profile_fields.display_name,
            dashpay_profile_bio: profile_fields.bio,
            dashpay_profile_avatar_url: profile_fields.avatar_url,
            dashpay_profile_avatar_hash: profile_fields.avatar_hash,
            dashpay_profile_avatar_hash_present: profile_fields.avatar_hash_present,
            dashpay_profile_avatar_fingerprint: profile_fields.avatar_fingerprint,
            dashpay_profile_avatar_fingerprint_present: profile_fields.avatar_fingerprint_present,
            dashpay_profile_public_message: profile_fields.public_message,
            contact_profiles,
            contact_profiles_count,
        }
    }
}

/// Intermediate carrier for the DashPay profile slice of
/// [`IdentityEntryFFI`]. Exists so [`IdentityEntryFFI::from_entry`]
/// can build the per-string heap allocations in one place without
/// open-coding the `Option<String>` → `CString::into_raw` ladder
/// inline. Every owned pointer in here is released by
/// [`free_identity_entry_ffi`] when the parent struct is freed.
struct DashPayProfileFields {
    present: bool,
    display_name: *const c_char,
    bio: *const c_char,
    avatar_url: *const c_char,
    avatar_hash: [u8; 32],
    avatar_hash_present: bool,
    avatar_fingerprint: [u8; 8],
    avatar_fingerprint_present: bool,
    public_message: *const c_char,
}

impl DashPayProfileFields {
    /// Zeroed/null carrier used when the source profile is `None`.
    fn absent() -> Self {
        Self {
            present: false,
            display_name: ptr::null(),
            bio: ptr::null(),
            avatar_url: ptr::null(),
            avatar_hash: [0u8; 32],
            avatar_hash_present: false,
            avatar_fingerprint: [0u8; 8],
            avatar_fingerprint_present: false,
            public_message: ptr::null(),
        }
    }

    /// Heap-allocate the C strings for a present profile. Strings
    /// containing interior NUL bytes (impossible in practice — the
    /// DashPay contract validation rejects them) become null
    /// pointers so the rest of the struct stays well-formed; Swift
    /// reads each pointer as nullable already.
    fn from_profile(profile: &DashPayProfile) -> Self {
        let (avatar_hash, avatar_hash_present) = match profile.avatar_hash {
            Some(h) => (h, true),
            None => ([0u8; 32], false),
        };
        let (avatar_fingerprint, avatar_fingerprint_present) = match profile.avatar_fingerprint {
            Some(f) => (f, true),
            None => ([0u8; 8], false),
        };
        Self {
            present: true,
            display_name: optional_c_string(profile.display_name.as_deref()),
            bio: optional_c_string(profile.bio.as_deref()),
            avatar_url: optional_c_string(profile.avatar_url.as_deref()),
            avatar_hash,
            avatar_hash_present,
            avatar_fingerprint,
            avatar_fingerprint_present,
            public_message: optional_c_string(profile.public_message.as_deref()),
        }
    }
}

/// Convert an `Option<&str>` into a heap-allocated `CString` raw
/// pointer (`null` for `None`). The returned pointer is released
/// with `CString::from_raw` inside [`free_identity_entry_ffi`].
fn optional_c_string(s: Option<&str>) -> *const c_char {
    match s {
        Some(s) => match CString::new(s) {
            Ok(c) => c.into_raw() as *const c_char,
            Err(_) => ptr::null(),
        },
        None => ptr::null(),
    }
}

/// Allocate the two parallel DPNS arrays carried on
/// [`IdentityEntryFFI`]. Returns `(labels, acquired_at, count)` —
/// both pointers null and count `0` when the source slice is empty.
///
/// `labels` is a `Box<[*const c_char]>` of `CString::into_raw`
/// pointers — release each entry with `CString::from_raw` before
/// dropping the outer slice. `acquired_at` is a `Box<[u64]>` of
/// matching Unix-millis timestamps (`0` for `None`). The two slices
/// always have the same length so the caller indexes them in
/// lock-step.
///
/// Inner labels that fail `CString::new` (interior NUL — unreachable
/// in practice given DPNS validation) become null entries so the
/// outer iteration on the Swift side stays index-aligned with the
/// timestamp array.
fn allocate_dpns_arrays(
    names: &[platform_wallet::DpnsNameInfo],
) -> (*const *const c_char, *const u64, usize) {
    if names.is_empty() {
        return (ptr::null(), ptr::null(), 0);
    }
    let mut labels: Vec<*const c_char> = Vec::with_capacity(names.len());
    let mut acquired: Vec<u64> = Vec::with_capacity(names.len());
    for info in names {
        let raw = match CString::new(info.label.clone()) {
            Ok(s) => s.into_raw() as *const c_char,
            // Interior NUL: skip the label but keep the slot so the
            // timestamp array stays index-aligned.
            Err(_) => ptr::null(),
        };
        labels.push(raw);
        acquired.push(info.acquired_at.unwrap_or(0));
    }
    let count = labels.len();
    let labels_ptr = Box::into_raw(labels.into_boxed_slice()) as *const *const c_char;
    let acquired_ptr = Box::into_raw(acquired.into_boxed_slice()) as *const u64;
    (labels_ptr, acquired_ptr, count)
}

/// Allocate the [`ContactProfileRowFFI`] array carried on
/// [`IdentityEntryFFI`] from the source
/// [`IdentityEntry::contact_profiles`] map. Returns `(rows, count)` —
/// both `null`/`0` when the map is empty. Every map entry produces a
/// row: present profiles as full rows, confirmed-absent entries
/// (`ContactProfileEntry::profile == None`) as `is_present == false`
/// tombstones that tell the consumer to DELETE its persisted row (a
/// contact who removed their profile must not keep showing a stale
/// name/avatar). `count` is therefore the map length.
///
/// `rows` is a `Box<[ContactProfileRowFFI]>` (via [`Box::into_raw`]).
/// Each row's four nullable C-strings are [`CString::into_raw`]
/// pointers — every one must be released with `CString::from_raw`
/// before the outer slice drops. [`free_identity_entry_ffi`] does this
/// row-by-row, mirroring the DPNS label-array free path exactly.
fn allocate_contact_profile_rows(
    contact_profiles: &std::collections::BTreeMap<
        dpp::prelude::Identifier,
        platform_wallet::ContactProfileEntry,
    >,
) -> (*const ContactProfileRowFFI, usize) {
    if contact_profiles.is_empty() {
        return (ptr::null(), 0);
    }
    let mut rows: Vec<ContactProfileRowFFI> = Vec::with_capacity(contact_profiles.len());
    for (contact_id, entry) in contact_profiles {
        // Confirmed-absent entry: emit an `is_present = false` tombstone row so
        // Swift DELETEs any persisted row for this contact. A contact who
        // removed their profile (present -> absent) must not keep showing a
        // stale name/avatar — skipping the entry would leave the old upserted
        // row untouched forever.
        let Some(profile) = entry.profile.as_ref() else {
            rows.push(ContactProfileRowFFI {
                contact_id: contact_id.to_buffer(),
                is_present: false,
                display_name: ptr::null(),
                bio: ptr::null(),
                avatar_url: ptr::null(),
                avatar_hash: [0u8; 32],
                avatar_hash_present: false,
                avatar_fingerprint: [0u8; 8],
                avatar_fingerprint_present: false,
                public_message: ptr::null(),
                checked_at_ms: entry.checked_at_ms,
            });
            continue;
        };
        let (avatar_hash, avatar_hash_present) = match profile.avatar_hash {
            Some(h) => (h, true),
            None => ([0u8; 32], false),
        };
        let (avatar_fingerprint, avatar_fingerprint_present) = match profile.avatar_fingerprint {
            Some(f) => (f, true),
            None => ([0u8; 8], false),
        };
        rows.push(ContactProfileRowFFI {
            contact_id: contact_id.to_buffer(),
            is_present: true,
            display_name: optional_c_string(profile.display_name.as_deref()),
            bio: optional_c_string(profile.bio.as_deref()),
            avatar_url: optional_c_string(profile.avatar_url.as_deref()),
            avatar_hash,
            avatar_hash_present,
            avatar_fingerprint,
            avatar_fingerprint_present,
            public_message: optional_c_string(profile.public_message.as_deref()),
            checked_at_ms: entry.checked_at_ms,
        });
    }
    // `rows` is non-empty here: the early return above covers the empty map,
    // and every entry (present or absent) now pushes a row.
    let count = rows.len();
    let rows_ptr = Box::into_raw(rows.into_boxed_slice()) as *const ContactProfileRowFFI;
    (rows_ptr, count)
}

impl IdentityKeyEntryFFI {
    /// Copy an [`IdentityKeyEntry`] into a fresh FFI struct. The
    /// caller owns the heap-allocated `public_key_data_ptr` byte
    /// buffer and (when present) the
    /// `contract_bounds_document_type` C-string; release both via
    /// [`free_identity_key_entry_ffi`].
    pub fn from_entry(entry: &IdentityKeyEntry) -> Self {
        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
        use dpp::identity::identity_public_key::contract_bounds::ContractBounds;

        let pk_bytes = entry.public_key.data().as_slice().to_vec();
        let pk_len = pk_bytes.len();
        let pk_boxed = pk_bytes.into_boxed_slice();
        let public_key_data_ptr = Box::into_raw(pk_boxed) as *mut u8;

        let (disabled_some, disabled_at) = match entry.public_key.disabled_at() {
            Some(ts) => (true, ts),
            None => (false, 0u64),
        };

        let (wallet_id_is_some, wallet_id) = match entry.wallet_id {
            Some(id) => (true, id),
            None => (false, [0u8; 32]),
        };

        let (derivation_indices_is_some, identity_index, key_index) = match entry.derivation_indices
        {
            Some(idx) => (true, idx.identity_index, idx.key_index),
            None => (false, 0, 0),
        };

        // Project the DPP `ContractBounds` enum into the kind /
        // id / doc-type-cstring trio so the Swift side can switch
        // on a single discriminant. Strings containing interior
        // NULs (impossible in practice — DPP rejects them) keep
        // the discriminant + payload self-consistent by falling
        // back to `SingleContract { id }` (kind=1 + null doc-type
        // pointer); emitting kind=2 with a null doc-type pointer
        // would silently strip the bound on the Swift side, so
        // demoting to `SingleContract` is the closest faithful
        // representation — the document-type qualifier is the
        // only thing lost, the contract id is preserved.
        let (contract_bounds_kind, contract_bounds_id, contract_bounds_document_type) =
            match entry.public_key.contract_bounds() {
                Some(ContractBounds::SingleContract { id }) => (1u8, id.to_buffer(), ptr::null()),
                Some(ContractBounds::SingleContractDocumentType {
                    id,
                    document_type_name,
                }) => match CString::new(document_type_name.as_str()) {
                    Ok(c) => (2u8, id.to_buffer(), c.into_raw() as *const c_char),
                    Err(_) => (1u8, id.to_buffer(), ptr::null()),
                },
                None => (0u8, [0u8; 32], ptr::null()),
            };

        Self {
            identity_id: entry.identity_id.to_buffer(),
            key_id: entry.key_id,
            purpose: entry.public_key.purpose() as u8,
            security_level: entry.public_key.security_level() as u8,
            key_type: entry.public_key.key_type() as u8,
            read_only: entry.public_key.read_only(),
            disabled_at_is_some: disabled_some,
            disabled_at,
            public_key_data_ptr,
            public_key_data_len: pk_len,
            public_key_hash: entry.public_key_hash,
            wallet_id_is_some,
            wallet_id,
            derivation_indices_is_some,
            identity_index,
            key_index,
            contract_bounds_kind,
            contract_bounds_id,
            contract_bounds_document_type,
        }
    }
}

/// Map an [`IdentityStatus`] onto its FFI discriminant. Hand-mapped so
/// the encoding stays stable even if the upstream enum grows new
/// variants (the default branch is the sentinel `0xFF = Unknown`).
fn status_discriminant(status: IdentityStatus) -> u8 {
    match status {
        IdentityStatus::Unknown => 0,
        IdentityStatus::PendingCreation => 1,
        IdentityStatus::Active => 2,
        IdentityStatus::FailedCreation => 3,
        IdentityStatus::NotFound => 4,
    }
}

// ---------------------------------------------------------------------------
// Destructors
// ---------------------------------------------------------------------------

/// Release heap allocations owned by an [`IdentityEntryFFI`] —
/// the DPNS label C-string array (each entry plus the outer boxed
/// slice), the parallel `acquired_at` timestamp array, (when
/// [`IdentityEntryFFI::dashpay_profile_present`] is true) the
/// own-profile per-string C-strings, and the cached contact-profile
/// row array (each row's four per-string C-strings plus the outer
/// boxed slice).
///
/// Idempotent: pointers are nulled, the `_present` flag is reset,
/// and counts are zeroed after release, so a second call is a no-op.
///
/// # Safety
///
/// `entry` must have been produced by [`IdentityEntryFFI::from_entry`]
/// and not previously freed. The pointers must reference allocations
/// owned by this struct — passing in pointers Swift owns or pointers
/// from a different allocator will corrupt the heap.
pub unsafe fn free_identity_entry_ffi(entry: &mut IdentityEntryFFI) {
    if !entry.dpns_names.is_null() && entry.dpns_names_count > 0 {
        // Reconstruct the boxed slice we created via `Box::into_raw`
        // on a `Box<[*const c_char]>`, then walk every entry to
        // release the per-label C-string before the outer slice
        // drops.
        let slice = unsafe {
            std::slice::from_raw_parts_mut(
                entry.dpns_names as *mut *const c_char,
                entry.dpns_names_count,
            )
        };
        for raw in slice.iter_mut() {
            if !raw.is_null() {
                let _ = unsafe { CString::from_raw(*raw as *mut c_char) };
                *raw = ptr::null();
            }
        }
        let _ = unsafe { Box::from_raw(slice as *mut [*const c_char]) };
        entry.dpns_names = ptr::null();
    }
    if !entry.dpns_names_acquired_at.is_null() && entry.dpns_names_count > 0 {
        let slice = unsafe {
            std::slice::from_raw_parts_mut(
                entry.dpns_names_acquired_at as *mut u64,
                entry.dpns_names_count,
            )
        };
        let _ = unsafe { Box::from_raw(slice as *mut [u64]) };
        entry.dpns_names_acquired_at = ptr::null();
    }
    entry.dpns_names_count = 0;

    // Release each per-string DashPay profile allocation. The
    // `_present` flag gates the whole section — when the source
    // profile was `None`, every pointer is already null and there
    // is nothing to free. We still walk each pointer individually
    // because a profile can be present with one or more
    // `Option<String>` fields unset (and therefore null).
    if entry.dashpay_profile_present {
        free_optional_c_string(&mut entry.dashpay_profile_display_name);
        free_optional_c_string(&mut entry.dashpay_profile_bio);
        free_optional_c_string(&mut entry.dashpay_profile_avatar_url);
        free_optional_c_string(&mut entry.dashpay_profile_public_message);
        entry.dashpay_profile_avatar_hash = [0u8; 32];
        entry.dashpay_profile_avatar_hash_present = false;
        entry.dashpay_profile_avatar_fingerprint = [0u8; 8];
        entry.dashpay_profile_avatar_fingerprint_present = false;
        entry.dashpay_profile_present = false;
    }

    // Release the cached contact-profile rows. Mirrors the DPNS
    // label-array free path: reconstruct the `Box<[ContactProfileRowFFI]>`
    // we created via `Box::into_raw`, walk every row to release its four
    // per-string `CString`s, then drop the outer slice. Each string was
    // produced by `optional_c_string` (`CString::into_raw`) so it MUST be
    // reclaimed with `CString::from_raw` — the byte arrays are inline and
    // need no free.
    if !entry.contact_profiles.is_null() && entry.contact_profiles_count > 0 {
        let rows = unsafe {
            std::slice::from_raw_parts_mut(
                entry.contact_profiles as *mut ContactProfileRowFFI,
                entry.contact_profiles_count,
            )
        };
        for row in rows.iter_mut() {
            free_optional_c_string(&mut row.display_name);
            free_optional_c_string(&mut row.bio);
            free_optional_c_string(&mut row.avatar_url);
            free_optional_c_string(&mut row.public_message);
        }
        let _ = unsafe { Box::from_raw(rows as *mut [ContactProfileRowFFI]) };
        entry.contact_profiles = ptr::null();
    }
    entry.contact_profiles_count = 0;
}

/// Release a heap-allocated C string produced by
/// [`optional_c_string`] and null out the pointer in place. Idempotent
/// for `null` inputs so [`free_identity_entry_ffi`] stays a no-op on
/// double calls.
///
/// # Safety
///
/// The pointer must either be `null` or have been produced by
/// `CString::into_raw` on a `Box`-allocated `CString` (i.e. the
/// system allocator) — the same allocator `CString::from_raw`
/// reclaims from.
unsafe fn free_optional_c_string(slot: &mut *const c_char) {
    if !slot.is_null() {
        let _ = unsafe { CString::from_raw(*slot as *mut c_char) };
        *slot = ptr::null();
    }
}

/// Release heap allocations owned by an [`IdentityKeyEntryFFI`] —
/// the public-key data buffer and, when present, the contract-bounds
/// document-type C-string (set when
/// `contract_bounds_kind == 2`, i.e. SingleContractDocumentType).
///
/// Idempotent: pointers are nulled and length zeroed after release,
/// so a second call is a no-op.
///
/// # Safety
///
/// `entry` must have been produced by
/// [`IdentityKeyEntryFFI::from_entry`] and not previously freed.
pub unsafe fn free_identity_key_entry_ffi(entry: &mut IdentityKeyEntryFFI) {
    if !entry.public_key_data_ptr.is_null() && entry.public_key_data_len > 0 {
        // Reconstruct the boxed slice we created via `Box::into_raw`
        // on a `Box<[u8]>`. Using `Vec::from_raw_parts` would over-
        // allocate because the slice's capacity equals its length
        // post-`into_boxed_slice`.
        let slice = unsafe {
            std::slice::from_raw_parts_mut(entry.public_key_data_ptr, entry.public_key_data_len)
        };
        let _ = unsafe { Box::from_raw(slice as *mut [u8]) };
        entry.public_key_data_ptr = ptr::null_mut();
        entry.public_key_data_len = 0;
    }
    // Release the contract-bounds doc-type C-string. Only allocated
    // when the original entry carried `SingleContractDocumentType`
    // bounds (and the doc-type name didn't contain interior NULs).
    if !entry.contract_bounds_document_type.is_null() {
        let _ = unsafe { CString::from_raw(entry.contract_bounds_document_type as *mut c_char) };
        entry.contract_bounds_document_type = ptr::null();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    use dpp::prelude::Identifier;
    use platform_wallet::changeset::{
        IdentityEntry, IdentityKeyDerivationIndices, IdentityKeyEntry,
    };

    #[test]
    fn test_identity_entry_ffi_round_trip() {
        let entry = IdentityEntry {
            id: Identifier::from([7u8; 32]),
            balance: 1_234_567,
            revision: 3,
            identity_index: Some(42),
            last_updated_balance_block_time: None,
            last_synced_keys_block_time: None,
            dpns_names: Vec::new(),
            contested_dpns_names: Vec::new(),
            status: IdentityStatus::Active,
            wallet_id: Some([9u8; 32]),
            dashpay_profile: None,
            dashpay_payments: Default::default(),
            contact_profiles: Default::default(),
            ignored_senders: Default::default(),
        };
        let mut ffi = IdentityEntryFFI::from_entry(&entry);
        assert_eq!(ffi.identity_id, [7u8; 32]);
        assert_eq!(ffi.balance, 1_234_567);
        assert_eq!(ffi.revision, 3);
        assert!(ffi.identity_index_is_some);
        assert_eq!(ffi.identity_index, 42);
        assert_eq!(ffi.status, 2); // Active
        assert!(ffi.wallet_id_is_some);
        assert_eq!(ffi.wallet_id, [9u8; 32]);
        assert!(ffi.dpns_names.is_null());
        assert!(ffi.dpns_names_acquired_at.is_null());
        assert_eq!(ffi.dpns_names_count, 0);
        unsafe { free_identity_entry_ffi(&mut ffi) };
    }

    #[test]
    fn test_identity_entry_ffi_with_dpns_names() {
        use platform_wallet::DpnsNameInfo;
        let entry = IdentityEntry {
            id: Identifier::from([4u8; 32]),
            balance: 0,
            revision: 0,
            identity_index: Some(0),
            last_updated_balance_block_time: None,
            last_synced_keys_block_time: None,
            dpns_names: vec![
                DpnsNameInfo {
                    label: "alice".to_string(),
                    acquired_at: Some(1_700_000_000_000),
                },
                DpnsNameInfo {
                    label: "alice2".to_string(),
                    acquired_at: None,
                },
            ],
            contested_dpns_names: Vec::new(),
            status: IdentityStatus::Active,
            wallet_id: None,
            dashpay_profile: None,
            dashpay_payments: Default::default(),
            contact_profiles: Default::default(),
            ignored_senders: Default::default(),
        };
        let mut ffi = IdentityEntryFFI::from_entry(&entry);
        assert_eq!(ffi.dpns_names_count, 2);
        assert!(!ffi.dpns_names.is_null());
        assert!(!ffi.dpns_names_acquired_at.is_null());

        // Read both labels back via the C-string API to validate the
        // shape Swift is going to walk.
        let labels: &[*const c_char] =
            unsafe { std::slice::from_raw_parts(ffi.dpns_names, ffi.dpns_names_count) };
        let acquired: &[u64] =
            unsafe { std::slice::from_raw_parts(ffi.dpns_names_acquired_at, ffi.dpns_names_count) };
        assert!(!labels[0].is_null());
        assert!(!labels[1].is_null());
        let s0 = unsafe { std::ffi::CStr::from_ptr(labels[0]) }
            .to_str()
            .unwrap();
        let s1 = unsafe { std::ffi::CStr::from_ptr(labels[1]) }
            .to_str()
            .unwrap();
        assert_eq!(s0, "alice");
        assert_eq!(s1, "alice2");
        assert_eq!(acquired[0], 1_700_000_000_000);
        assert_eq!(acquired[1], 0);

        unsafe { free_identity_entry_ffi(&mut ffi) };
        assert!(ffi.dpns_names.is_null());
        assert!(ffi.dpns_names_acquired_at.is_null());
        assert_eq!(ffi.dpns_names_count, 0);

        // Idempotent: a second call must not double-free.
        unsafe { free_identity_entry_ffi(&mut ffi) };
    }

    #[test]
    fn test_identity_entry_ffi_with_dashpay_profile() {
        use platform_wallet::DashPayProfile;
        let entry = IdentityEntry {
            id: Identifier::from([5u8; 32]),
            balance: 0,
            revision: 0,
            identity_index: Some(1),
            last_updated_balance_block_time: None,
            last_synced_keys_block_time: None,
            dpns_names: Vec::new(),
            contested_dpns_names: Vec::new(),
            status: IdentityStatus::Active,
            wallet_id: None,
            dashpay_profile: Some(DashPayProfile {
                display_name: Some("Bob".to_string()),
                bio: Some("Hello".to_string()),
                avatar_url: Some("https://example.com/a.png".to_string()),
                avatar_hash: Some([0xAB; 32]),
                avatar_fingerprint: Some([0xCD; 8]),
                public_message: None,
            }),
            dashpay_payments: Default::default(),
            contact_profiles: Default::default(),
            ignored_senders: Default::default(),
        };
        let mut ffi = IdentityEntryFFI::from_entry(&entry);
        assert!(ffi.dashpay_profile_present);
        let display = unsafe { std::ffi::CStr::from_ptr(ffi.dashpay_profile_display_name) }
            .to_str()
            .unwrap();
        assert_eq!(display, "Bob");
        let bio = unsafe { std::ffi::CStr::from_ptr(ffi.dashpay_profile_bio) }
            .to_str()
            .unwrap();
        assert_eq!(bio, "Hello");
        let url = unsafe { std::ffi::CStr::from_ptr(ffi.dashpay_profile_avatar_url) }
            .to_str()
            .unwrap();
        assert_eq!(url, "https://example.com/a.png");
        assert!(ffi.dashpay_profile_avatar_hash_present);
        assert_eq!(ffi.dashpay_profile_avatar_hash, [0xAB; 32]);
        assert!(ffi.dashpay_profile_avatar_fingerprint_present);
        assert_eq!(ffi.dashpay_profile_avatar_fingerprint, [0xCD; 8]);
        assert!(ffi.dashpay_profile_public_message.is_null());

        unsafe { free_identity_entry_ffi(&mut ffi) };
        assert!(!ffi.dashpay_profile_present);
        assert!(ffi.dashpay_profile_display_name.is_null());
        assert!(ffi.dashpay_profile_bio.is_null());
        assert!(ffi.dashpay_profile_avatar_url.is_null());
        assert!(!ffi.dashpay_profile_avatar_hash_present);
        assert!(!ffi.dashpay_profile_avatar_fingerprint_present);
        // Idempotent — second call must not double-free.
        unsafe { free_identity_entry_ffi(&mut ffi) };
    }

    #[test]
    fn contact_profile_rows_emit_present_and_absent_tombstone() {
        use platform_wallet::{ContactProfileEntry, DashPayProfile};
        // [7;32] (present) sorts before [8;32] (absent) in the BTreeMap, so the
        // emitted rows are in that order.
        let mut contact_profiles = std::collections::BTreeMap::new();
        contact_profiles.insert(
            Identifier::from([7u8; 32]),
            ContactProfileEntry {
                profile: Some(DashPayProfile {
                    display_name: Some("Carol".to_string()),
                    bio: None,
                    avatar_url: None,
                    avatar_hash: Some([0x11; 32]),
                    avatar_fingerprint: None,
                    public_message: None,
                }),
                checked_at_ms: 111,
            },
        );
        contact_profiles.insert(
            Identifier::from([8u8; 32]),
            ContactProfileEntry {
                profile: None,
                checked_at_ms: 222,
            },
        );
        let entry = IdentityEntry {
            id: Identifier::from([9u8; 32]),
            balance: 0,
            revision: 0,
            identity_index: Some(1),
            last_updated_balance_block_time: None,
            last_synced_keys_block_time: None,
            dpns_names: Vec::new(),
            contested_dpns_names: Vec::new(),
            status: IdentityStatus::Active,
            wallet_id: None,
            dashpay_profile: None,
            dashpay_payments: Default::default(),
            contact_profiles,
            ignored_senders: Default::default(),
        };
        let mut ffi = IdentityEntryFFI::from_entry(&entry);

        // A confirmed-absent entry now rides as an `is_present == false`
        // tombstone instead of being dropped — both entries are projected.
        assert_eq!(ffi.contact_profiles_count, 2);
        let rows =
            unsafe { std::slice::from_raw_parts(ffi.contact_profiles, ffi.contact_profiles_count) };

        let present = &rows[0];
        assert_eq!(present.contact_id, [7u8; 32]);
        assert!(present.is_present);
        assert_eq!(
            unsafe { std::ffi::CStr::from_ptr(present.display_name) }
                .to_str()
                .unwrap(),
            "Carol"
        );
        assert!(present.avatar_hash_present);
        assert_eq!(present.avatar_hash, [0x11; 32]);
        assert!(!present.avatar_fingerprint_present);
        assert_eq!(present.checked_at_ms, 111);

        let absent = &rows[1];
        assert_eq!(absent.contact_id, [8u8; 32]);
        assert!(!absent.is_present);
        assert!(absent.display_name.is_null());
        assert!(absent.bio.is_null());
        assert!(absent.avatar_url.is_null());
        assert!(absent.public_message.is_null());
        assert!(!absent.avatar_hash_present);
        assert!(!absent.avatar_fingerprint_present);
        // The self-heal timestamp must still ride so Swift can delete the stale
        // row and the negative-cache backoff is preserved.
        assert_eq!(absent.checked_at_ms, 222);

        unsafe { free_identity_entry_ffi(&mut ffi) };
        assert!(ffi.contact_profiles.is_null());
        assert_eq!(ffi.contact_profiles_count, 0);
        // Idempotent — second free (null strings on the tombstone included)
        // must not double-free.
        unsafe { free_identity_entry_ffi(&mut ffi) };
    }

    #[test]
    fn test_identity_entry_ffi_no_wallet() {
        let entry = IdentityEntry {
            id: Identifier::from([1u8; 32]),
            balance: 0,
            revision: 0,
            identity_index: None,
            last_updated_balance_block_time: None,
            last_synced_keys_block_time: None,
            dpns_names: Vec::new(),
            contested_dpns_names: Vec::new(),
            status: IdentityStatus::Unknown,
            wallet_id: None,
            dashpay_profile: None,
            dashpay_payments: Default::default(),
            contact_profiles: Default::default(),
            ignored_senders: Default::default(),
        };
        let mut ffi = IdentityEntryFFI::from_entry(&entry);
        assert!(!ffi.wallet_id_is_some);
        assert_eq!(ffi.wallet_id, [0u8; 32]);
        assert!(!ffi.identity_index_is_some);
        assert_eq!(ffi.identity_index, 0);
        assert_eq!(ffi.status, 0); // Unknown
        unsafe { free_identity_entry_ffi(&mut ffi) };
    }

    #[test]
    fn test_identity_key_entry_ffi_with_derivation_indices() {
        let public_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 5,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![0xAB; 33]),
            disabled_at: None,
        });
        let entry = IdentityKeyEntry {
            identity_id: Identifier::from([2u8; 32]),
            key_id: 5,
            public_key,
            public_key_hash: [0x77; 20],
            wallet_id: Some([0x9A; 32]),
            derivation_indices: Some(IdentityKeyDerivationIndices {
                identity_index: 3,
                key_index: 5,
            }),
        };
        let mut ffi = IdentityKeyEntryFFI::from_entry(&entry);
        assert_eq!(ffi.identity_id, [2u8; 32]);
        assert_eq!(ffi.key_id, 5);
        assert_eq!(ffi.purpose, Purpose::AUTHENTICATION as u8);
        assert_eq!(ffi.security_level, SecurityLevel::HIGH as u8);
        assert_eq!(ffi.key_type, KeyType::ECDSA_SECP256K1 as u8);
        assert!(!ffi.read_only);
        assert!(!ffi.disabled_at_is_some);
        assert_eq!(ffi.public_key_data_len, 33);
        let data_slice =
            unsafe { std::slice::from_raw_parts(ffi.public_key_data_ptr, ffi.public_key_data_len) };
        assert_eq!(data_slice, &[0xAB; 33]);
        assert_eq!(ffi.public_key_hash, [0x77; 20]);
        assert!(ffi.wallet_id_is_some);
        assert_eq!(ffi.wallet_id, [0x9A; 32]);
        assert!(ffi.derivation_indices_is_some);
        assert_eq!(ffi.identity_index, 3);
        assert_eq!(ffi.key_index, 5);
        unsafe { free_identity_key_entry_ffi(&mut ffi) };
        assert!(ffi.public_key_data_ptr.is_null());
    }

    #[test]
    fn test_identity_key_entry_ffi_watch_only() {
        let public_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::ENCRYPTION,
            security_level: SecurityLevel::MEDIUM,
            contract_bounds: None,
            key_type: KeyType::ECDSA_HASH160,
            read_only: true,
            data: BinaryData::new(vec![0x12; 20]),
            disabled_at: Some(1_700_000_000),
        });
        let entry = IdentityKeyEntry {
            identity_id: Identifier::from([3u8; 32]),
            key_id: 0,
            public_key,
            public_key_hash: [0x00; 20],
            wallet_id: None,
            derivation_indices: None,
        };
        let mut ffi = IdentityKeyEntryFFI::from_entry(&entry);
        assert!(!ffi.wallet_id_is_some);
        assert!(!ffi.derivation_indices_is_some);
        assert!(ffi.read_only);
        assert!(ffi.disabled_at_is_some);
        assert_eq!(ffi.disabled_at, 1_700_000_000);
        assert_eq!(ffi.contract_bounds_kind, 0);
        assert!(ffi.contract_bounds_document_type.is_null());
        unsafe { free_identity_key_entry_ffi(&mut ffi) };
    }

    #[test]
    fn test_identity_key_entry_ffi_contract_bounds_single_contract() {
        use dpp::identity::identity_public_key::contract_bounds::ContractBounds;
        let contract_id = Identifier::from([0xAB; 32]);
        let public_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: Some(ContractBounds::SingleContract { id: contract_id }),
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![0x01; 33]),
            disabled_at: None,
        });
        let entry = IdentityKeyEntry {
            identity_id: Identifier::from([1u8; 32]),
            key_id: 1,
            public_key,
            public_key_hash: [0x11; 20],
            wallet_id: None,
            derivation_indices: None,
        };
        let mut ffi = IdentityKeyEntryFFI::from_entry(&entry);
        assert_eq!(ffi.contract_bounds_kind, 1);
        assert_eq!(ffi.contract_bounds_id, [0xAB; 32]);
        assert!(ffi.contract_bounds_document_type.is_null());
        unsafe { free_identity_key_entry_ffi(&mut ffi) };
    }

    #[test]
    fn test_identity_key_entry_ffi_contract_bounds_single_doc_type() {
        use dpp::identity::identity_public_key::contract_bounds::ContractBounds;
        let contract_id = Identifier::from([0xCD; 32]);
        let public_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 2,
            purpose: Purpose::ENCRYPTION,
            security_level: SecurityLevel::MEDIUM,
            contract_bounds: Some(ContractBounds::SingleContractDocumentType {
                id: contract_id,
                document_type_name: "contactRequest".to_string(),
            }),
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![0x02; 33]),
            disabled_at: None,
        });
        let entry = IdentityKeyEntry {
            identity_id: Identifier::from([4u8; 32]),
            key_id: 2,
            public_key,
            public_key_hash: [0x22; 20],
            wallet_id: None,
            derivation_indices: None,
        };
        let mut ffi = IdentityKeyEntryFFI::from_entry(&entry);
        assert_eq!(ffi.contract_bounds_kind, 2);
        assert_eq!(ffi.contract_bounds_id, [0xCD; 32]);
        assert!(!ffi.contract_bounds_document_type.is_null());
        // Verify the doc-type CString round-trips.
        let cstr = unsafe { std::ffi::CStr::from_ptr(ffi.contract_bounds_document_type) };
        assert_eq!(cstr.to_str().unwrap(), "contactRequest");
        unsafe { free_identity_key_entry_ffi(&mut ffi) };
        // Idempotent free.
        assert!(ffi.contract_bounds_document_type.is_null());
        unsafe { free_identity_key_entry_ffi(&mut ffi) };
    }
}
