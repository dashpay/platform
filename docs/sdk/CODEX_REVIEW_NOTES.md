# Independent review: Kotlin DashPay registration keys

**Review date:** 2026-07-20

**Reviewed branch/worktree:** `feat/kotlin-sdk-dashpay-registration-keys`

**Reviewed input:** `KOTLIN_SDK_DASHPAY_REGISTRATION_KEYS_SPEC.md`

## Bottom line

The underlying defect is real, the proposed PR split is sensible, and the main
design direction (send rich key rows over JNI and provision the DashPay
ENCRYPTION/DECRYPTION pair before identity creation) is sound. This is not a
consensus-protocol change and the existing C `IdentityPubkeyFFI` layout is
already sufficient.

I would not implement the spec exactly as written yet. The following material
points need reconciliation first:

1. Promoting only `DecodedPubkeyRow` and `decode_update_pubkeys_blob` to
   `pub(crate)` is insufficient: every field on the row is still private.
2. The key-id-0 invariant must not be added unconditionally to the shared
   update decoder. Identity updates legitimately add rows without key 0.
3. CREATE lacks the explicit contract-bounds *validation* call used by UPDATE,
   but Drive's CREATE insertion path still resolves the contract/document type
   and checks its bounded-key requirement. The spec's "any wrong constant
   registers successfully" and "exact Add Contact bug" failure description is
   therefore inaccurate.
4. Adding two keys changes the creation fee and exactly reaches the protocol's
   six-key maximum. Swift intentionally excludes the unused-asset-lock/resume
   path, while this spec includes it. Kotlin does not expose the tracked lock's
   amount, so a four-key-minimum lock may be insufficient for a six-key create.
5. The C/JNI function signatures remain stable, but the opaque Kotlin-to-JNI
   byte format changes incompatibly and the affected Kotlin registration APIs
   are public. This is a logical ABI/API concern even though it is not a
   protocol or C-struct change.
6. The proposed tests do not yet prove cross-language wire compatibility or
   the on-chain contract bounds, and one verification command names a
   nonexistent Cargo package.

## 1. Technical-claim verification

### 1.1 Bug and current registration paths: verified

- `role_for_registration_key_id` assigns roles solely from key ID and has no
  ENCRYPTION/DECRYPTION case
  (`packages/rs-unified-sdk-jni/src/identity.rs:77-131`).
- The legacy decoder carries only `keyId` and public bytes
  (`packages/rs-unified-sdk-jni/src/identity.rs:930-1003`).
- All four JNI paths reconstruct the role, set `read_only: false`, and set
  `contract_bounds_kind: 0`:

  - resume: `packages/rs-unified-sdk-jni/src/identity.rs:599-623`
  - Core/asset-lock funded: `packages/rs-unified-sdk-jni/src/identity.rs:732-767`
  - Platform-address funded: `packages/rs-unified-sdk-jni/src/identity.rs:846-882`
  - shielded: `packages/rs-unified-sdk-jni/src/funding.rs:703-748`

- The Add Contact send path calls `select_own_encryption_key`
  (`packages/rs-platform-wallet/src/wallet/identity/network/contact_requests.rs:455-465`),
  which requires an enabled ECDSA_SECP256K1 ENCRYPTION key and otherwise emits
  the quoted failure (`contact_requests.rs:979-1005`). A freshly registered
  Kotlin identity therefore cannot pass that flow today.

### 1.2 Rich decoder reuse: feasible, but the spec omits required changes

The proposed parser is the correct *wire-format* reuse target:

- `DecodedPubkeyRow` and `decode_update_pubkeys_blob` are currently private at
  `packages/rs-unified-sdk-jni/src/transactions.rs:290-305`.
- The decoder reads key type, purpose, security level, read-only, bounds kind,
  public key, optional contract ID, and optional document type
  (`transactions.rs:318-444`).
- Update converts those owned rows into `IdentityPubkeyFFI` while retaining the
  owners through the synchronous FFI call (`transactions.rs:242-285`).
- Kotlin's encoder matches that layout
  (`packages/kotlin-sdk/sdk/src/main/kotlin/org/dashfoundation/dashsdk/identity/IdentityUpdates.kt:203-240`).

However, changing the struct and function to `pub(crate)` alone will not let
`identity.rs` or `funding.rs` build FFI rows: all fields remain private at
`transactions.rs:292-300`. Either expose the fields crate-wide or, preferably,
provide a crate-private conversion method/helper that keeps pointer creation
and owner lifetimes in one place.

The decoder should also be split into a pure `&[u8]` parser plus the thin JNI
exception adapter. The current function requires `JNIEnv`/`JByteArray`
(`transactions.rs:305-306`), which makes the proposed ordinary Rust round-trip
unit test unnecessarily difficult.

### 1.3 FFI bounds support: verified

- `IdentityPubkeyFFI` already contains all rich fields; no struct-layout change
  is needed
  (`packages/rs-platform-wallet-ffi/src/identity_registration_with_signer.rs:110-127`).
- `decode_contract_bounds` is correctly described as the wrong raw-JNI reuse
  target: it is `pub(crate)` in another crate and consumes an already-formed
  `&IdentityPubkeyFFI` (`identity_registration_with_signer.rs:129-160`).
- It rejects bounds kind 0 for ENCRYPTION/DECRYPTION
  (`identity_registration_with_signer.rs:161-173`) and maps kinds 1/2 at
  `:175-225`.
- `decode_identity_pubkeys` validates discriminants, public-key pointers, and
  contract bounds before constructing DPP keys
  (`identity_registration_with_signer.rs:273-323`). All four Android routes
  eventually call it (funded/resume in
  `identity_registration_funded_with_signer.rs:63-90,180-220`, address-funded
  at `identity_registration_with_signer.rs:434-470`, and shielded at
  `shielded_send.rs:663-720`).

One scope detail is missing: `decode_identity_pubkeys` has a fifth caller, the
Swift invitation claim path
(`packages/rs-platform-wallet-ffi/src/invitation.rs:244-300`). Moving a new
key-0 check into this FFI decoder affects that path too. Invitation already has
the same downstream invariant at
`packages/rs-platform-wallet/src/wallet/identity/network/invitation.rs:127-153`,
so the behavioral change should be benign, but it needs an explicit test and
should not be described as touching only four paths.

### 1.4 Key ID 0 pre-flight gap: verified, with a placement warning

- The shared funded/resume implementation rejects an empty map and requires
  `keys_map[0]` to be MASTER + AUTHENTICATION
  (`packages/rs-platform-wallet/src/wallet/identity/network/registration.rs:134-159`).
- Address-funded registration builds an identity directly from the decoded map
  and calls `register_from_addresses` without the same check
  (`packages/rs-platform-wallet-ffi/src/identity_registration_with_signer.rs:437-470`).
- Shielded registration converts the decoded map to a vector and proceeds
  without it (`packages/rs-platform-wallet-ffi/src/shielded_send.rs:703-720`).

This is specifically a **key ID 0** invariant, not "row 0" or "the first row."
DPP requires exactly one MASTER but does not require that master's ID to be 0.
The ID convention matters to wallet derivation and loading (`MASTER_KEY_INDEX`
is 0 in
`packages/rs-platform-wallet/src/wallet/identity/network/identity_handle.rs:55`).

Do not put this assertion directly in `decode_update_pubkeys_blob`: that parser
is used by `updateIdentity` (`transactions.rs:193-196`), and a normal add-key
update neither includes nor should include key 0. Safe options are:

- validate after decoding in a registration-only JNI wrapper used by the four
  registration exports; or
- validate in FFI `decode_identity_pubkeys`, acknowledging/testing the
  invitation caller as well.

### 1.5 DPP structure and proof-of-possession claims: mostly verified

`validate_identity_public_keys_structure_v0` does enforce:

- ENCRYPTION = MEDIUM only, DECRYPTION = MEDIUM only, TRANSFER = CRITICAL only
  (`packages/rs-dpp/src/state_transition/state_transitions/identity/public_key_in_creation/methods/validate_identity_public_keys_structure/v0/mod.rs:21-37,118-153`);
- duplicate key IDs/data (`:70-95`);
- exactly one MASTER for identity creation (`:98-116`); and
- at most six creation keys via the active platform version (`:49-68` and
  `packages/rs-platform-version/src/version/dpp_versions/dpp_state_transition_versions/v1.rs:18`,
  with the same value in v2/v3).

Proof of possession is also server-side, but the spec overgeneralizes the
function name. Standard asset-lock create verifies every key in
`identity_create/identity_and_signatures/v0/mod.rs:20-40`; address-funded create
uses its separate validator in
`identity_create_from_addresses/public_key_signatures/v0/mod.rs:20-42`; and
shielded create checks structure plus every PoP in
`packages/rs-drive-abci/src/execution/validation/state_transition/processor/traits/shielded_proof.rs:384-415`.

The security conclusion is correct: caller-controlled purpose/security bytes
cannot create an elevated ENCRYPTION/DECRYPTION or invalid TRANSFER key.
However, the statement that a malformed duplicate row list necessarily reaches
DPP and fails is false at the FFI boundary: `decode_identity_pubkeys` inserts
into a `BTreeMap` without checking the previous value
(`identity_registration_with_signer.rs:279-320`), so a later duplicate ID
silently overwrites the earlier row. The surviving key is still structurally
validated, so this does not create the claimed privilege escalation, but the
decoder should reject duplicate IDs explicitly.

### 1.6 CREATE-time contract-bounds semantics: the spec is only partly right

The narrow call-graph claim is true: the explicit
`validate_identity_public_keys_contract_bounds` state validator is called by
identity UPDATE
(`packages/rs-drive-abci/src/execution/validation/state_transition/state_transitions/identity_update/state/v0/mod.rs:97-110`),
not by standard or address identity CREATE. Those CREATE advanced-structure
paths call structural validation and PoP only
(`identity_create/advanced_structure/v0/mod.rs:80-116` and
`identity_create_from_addresses/advanced_structure/v0/mod.rs:34-71`). Shielded
CREATE likewise has only the structure/PoP checks cited above.

The stronger conclusion in the spec is not true. CREATE execution stores the
new identity through `create_key_tree_with_keys_operations`
(`packages/rs-drive/src/drive/identity/insert/add_new_identity/v0/mod.rs:259-270`).
Each key insertion calls
`add_potential_contract_info_for_contract_bounded_key`
(`packages/rs-drive/src/drive/identity/key/insert/create_key_tree_with_keys/v0/mod.rs:98-123`).
That path:

- fetches the referenced contract and errors if it does not exist
  (`packages/rs-drive/src/drive/identity/contract_info/keys/mod.rs:62-86`);
- resolves the named document type (`keys/mod.rs:94-105`); and
- requires that document type to declare the matching ENCRYPTION/DECRYPTION
  bounded-key requirement
  (`packages/rs-drive/src/drive/identity/contract_info/keys/add_potential_contract_info_for_contract_bounded_key/v0/mod.rs:430-465`).

Therefore an arbitrary nonexistent contract ID, missing document type, or
document without the relevant requirement cannot simply register
"successfully." A wrong but existing compatible contract/document could still
be accepted and indexed under the wrong scope, so pinning the constant remains
valuable.

It also would not recreate the *exact* selector failure described in the spec.
`select_own_encryption_key` tests only enabled/type/purpose, not bounds
(`contact_requests.rs:989-1005`), and contact-request validation intentionally
accepts bound or unbound ENCRYPTION keys
(`packages/rs-platform-wallet/src/wallet/identity/crypto/validation.rs:111-121,164-167`).
The contact document is built from the canonical DashPay contract fetched
separately (`packages/rs-sdk/src/platform/dashpay/contact_request.rs:341-349`).
Consequently, a manual Add Contact success is not proof that the new keys carry
the correct contract ID/document bounds.

### 1.7 Kotlin and Swift claims: verified with two material qualifications

- `IdentityUpdates.kt` already has the desired `IdentityPubkey`, enums, and
  bounds model (`:12-136`). `encodeAddPubkeys` is private at `:211`, and its
  helper `contractBoundsKind` is also private at `:243-248`; both need factoring
  if the encoder is shared.
- The Kotlin DashPay entry is private and its 32 bytes match
  `packages/dashpay-contract/src/lib.rs:9-16`
  (`packages/kotlin-sdk/KotlinExampleApp/app/src/main/java/org/dashfoundation/example/ui/identity/AddIdentityKeyScreen.kt:381-397`).
- Swift's helper creates consecutive ENCRYPTION/DECRYPTION ECDSA keys at MEDIUM
  with DashPay `contactRequest` bounds, validates private/public correspondence,
  persists each private key, and returns rich rows
  (`packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/Services/IdentityRegistrationKeys.swift:15-121`).

Qualification 1: the duplicate helper in `CreateIdentityView.swift` is a mirror,
not literally byte-for-byte "verbatim." That is editorial, not architectural.

Qualification 2: Swift deliberately excludes unused-asset-lock/resume from
DashPay provisioning (`CreateIdentityView.swift:152-177,896-905`). It also
raises the minimum funding according to total key count (`:127-149`). The Kotlin
flow proposes six keys on all four routes, including resume, but Kotlin's
`TrackedAssetLock` omits the Rust lock's `amount`
(`packages/kotlin-sdk/sdk/src/main/kotlin/org/dashfoundation/dashsdk/wallet/TrackedAssetLock.kt:11-19`
versus `packages/rs-platform-wallet/src/wallet/asset_lock/tracked.rs:63-72`).
The two added keys cost 13,000 additional duffs at the current 6,500,000-credit
per-key fee (`packages/rs-platform-version/src/version/fee/state_transition_min_fees/v1.rs:17`).

The spec must decide whether resume excludes the pair (matching Swift) or gains
an amount-aware sufficiency check/error. A previously built lock funded only
for four keys may fail a six-key create.

### 1.8 Deletion/caller audit: safe, but clean up related documentation

A repository-wide search found executable references to
`role_for_registration_key_id`/`decode_pubkeys_blob` only at the four JNI sites
listed above plus the `funding.rs:42` import. No test calls either function, so
deletion after replacement is safe. The six local role constants at
`identity.rs:79-90` then become dead as well.

The change should also update stale wire/positional-role documentation at:

- `packages/rs-unified-sdk-jni/src/identity.rs:258-270,682-697,813-815`
- `packages/rs-unified-sdk-jni/src/funding.rs:650-660`
- `packages/rs-platform-wallet-ffi/src/identity_key_preview.rs:77-94,257-280`
- `packages/kotlin-sdk/sdk/src/main/kotlin/org/dashfoundation/dashsdk/ffi/IdentityNative.kt:148-178`
- `packages/kotlin-sdk/sdk/src/main/kotlin/org/dashfoundation/dashsdk/identity/IdentityRegistration.kt:191-210`

## 2. Scope and compatibility assessment

### What is contained

- No consensus rule or serialized state-transition format needs changing.
- No C `IdentityPubkeyFFI` field/layout change is needed.
- No Room/database migration is needed; the existing identity-key persistence
  model already supports purpose/security/bounds.
- Separating this from Keystore invalidation/documentation follow-ups is a good
  rollback boundary: this change spans the registration wire codec and key
  provisioning, not storage invalidation.

### Hidden scope/risk to make explicit

1. **Opaque wire ABI.** The `byte[]` payload changes from
   `count + (id,len,pubkey)` to the rich update layout. JNI method descriptors
   remain identical, but an old Kotlin artifact paired with a new `.so` (or the
   reverse) will be misparsed/rejected. Update both sides and their KDocs in one
   release. A version/magic byte would make skew diagnosable; if that is deemed
   too broad, at least reject legacy/trailing input deterministically.
2. **Public Kotlin API.** `IdentityKeyPreview` is public
   (`IdentityKeyPreview.kt:17-22`), and public registration methods accept
   `List<IdentityKeyPreview>` (`IdentityRegistration.kt:117-146,237-291` and
   `PlatformWalletManager.kt:1344-1371`). Replacing that type, or adding fields
   to its data-class constructor, has source/binary/behavior compatibility
   implications for SDK consumers. Specify the compatibility strategy rather
   than treating it as an app-only row refactor.
3. **Six-key ceiling.** Base four plus the pair equals, rather than merely stays
   below, `max_public_keys_in_creation = 6`. Add a local assertion/test and
   document that any future expansion of the base set requires redesign.
4. **Existing identities.** This fixes only freshly registered identities.
   Existing Android-created identities remain without the pair and still need
   the Add Identity Key repair flow. State explicitly that no backfill is in
   scope, or add product guidance/detection if the intended outcome covers
   existing users.
5. **Existing Kotlin reuse.** The app already has derive -> validate -> persist
   -> zero -> `IdentityPubkey` plumbing in
   `packages/kotlin-sdk/KotlinExampleApp/app/src/main/java/org/dashfoundation/example/services/IdentityKeyAdditionFlow.kt:84-182`,
   used by `AddIdentityKeyScreen.kt:324-349`. Before introducing a separate
   `DashpayKeyProvisioning`, decide whether a thin policy wrapper around this
   helper is sufficient. Duplicating the lifecycle is another drift surface.
6. **Keypair validation parity.** Swift explicitly verifies the derived private
   scalar matches the public key (`IdentityRegistrationKeys.swift:66-82`). The
   current Kotlin registration preview path persists without this check
   (`CreateIdentityScreen.kt:202-235`). This is existing behavior, not caused by
   the pair, but a claimed Swift port should either add the defense or document
   the intentional parity gap.

## 3. Test-plan review and recommended additions

The device/testnet Add Contact smoke remains useful as an end-to-end product
check, but it is not enough and should not be the only integration evidence.
The following can be automated:

1. **One cross-language golden blob.** Have a Kotlin unit test encode six
   canonical rows to exact bytes, and have a pure Rust `&[u8]` parser test decode
   the same checked-in fixture. Independent Kotlin-object and Rust-object tests
   do not catch byte-order/field-order skew.
2. **Strict Rust codec tests.** Cover both bounds kinds, truncation at each
   variable field, invalid bounds kind, interior NUL, negative key ID, invalid
   boolean byte, duplicate ID, trailing bytes, and explicit legacy-format
   rejection. The current parser accepts every nonzero `read_only` byte as true
   and ignores trailing bytes (`transactions.rs:369-444`); decide and test a
   strict policy while this becomes a registration boundary.
3. **Registration-only invariant test.** Assert key ID 0 (regardless of row
   order) is MASTER + AUTHENTICATION; missing/wrong ID 0 fails. Add a separate
   regression proving identity-update add lists without key 0 remain accepted.
4. **FFI decoder test.** Verify duplicate IDs are rejected rather than
   overwritten and ENCRYPTION/DECRYPTION without bounds are rejected. If the
   key-0 check lives here, include the invitation caller behavior.
5. **Kotlin provisioning/lifecycle test.** Assert exact IDs 4/5, ECDSA type,
   ENCRYPTION/DECRYPTION purposes, MEDIUM level, `readOnly=false`, exact
   contract/document bounds, correct derivation indices, storage under the
   matching public bytes, and private-array zeroing on success and failure.
6. **Four-way orchestration test.** Extract a small pure preparation/dispatch
   seam from `CreateIdentityScreen` and verify all selected funding sources
   receive the same rich six-row set. This catches omission at a call site more
   cheaply than four JNI/JVM environments.
7. **On-chain bounds assertion.** After a local/testnet create, fetch the
   identity and assert IDs 4/5 carry the exact DashPay ID and `contactRequest`
   bounds (or query the contract-bound key index). Add Contact success alone
   does not test those fields.
8. **Resume funding case.** Test the chosen policy: either four keys for resume,
   or a clear early failure/success boundary based on the lock's available
   amount for six keys.
9. **Canonical constant test mechanism.** The proposed "Rust test" cannot
   compare a private Kotlin constant to `dashpay_contract::ID_BYTES`; Rust
   cannot see Kotlin source, and `rs-unified-sdk-jni` does not currently depend
   directly on `dashpay-contract`. Use generated Kotlin source/resource, a
   shared checked-in cross-language golden fixture, or a Kotlin test that
   consumes an independently generated canonical value. Merely copying the
   same 32-byte literal into both sides is the tautology the spec wants to
   avoid.

Existing Kotlin tests also need mechanical adjustment if the registration row
type/signature changes: `IdentityAssetLockRecoveryTest.kt:169` constructs the
legacy `IdentityKeyPreview` used by resume tests.

## 4. Verification-plan corrections

- `cargo test -p rs-platform-wallet-ffi --lib` is invalid: the package is named
  `platform-wallet-ffi` (`packages/rs-platform-wallet-ffi/Cargo.toml:1-2`). The
  correct command is `cargo test -p platform-wallet-ffi --lib`.
- Keep targeted tests first, then the proposed workspace clippy/format and
  Gradle suites. If production logic is added below the FFI crate, include the
  directly affected wallet/DPP/Drive package tests rather than relying only on
  JNI and FFI tests.
- Baseline source-head results from this review:

  - `cargo test -p rs-unified-sdk-jni --lib`: 10 passed, 0 failed.
  - `cargo test -p platform-wallet-ffi --lib`: 196 passed, 0 failed.
  - The spec's uncorrected `cargo test -p rs-platform-wallet-ffi --lib`
    invocation fails before testing because no package has that name.

The source audit did not modify production code or the reviewed spec.

## 5. Recommended spec edits before implementation

1. Specify a pure shared rich parser plus a safe FFI-row conversion API; mention
   row-field visibility/ownership explicitly.
2. State exactly where registration-only key-ID-0 validation lives and protect
   the update path with a regression test.
3. Rewrite the CREATE-bounds risk: no explicit consensus validator, but Drive
   insertion resolves and checks bounds; the residual risk is a wrong *existing
   compatible* scope and incorrect indexing, not necessarily Add Contact
   selector failure.
4. Decide the resume policy and account for the two-key fee plus six-key ceiling.
5. Document the logical JNI blob compatibility and public Kotlin API strategy.
6. Replace the constant-pin and round-trip bullets with an executable
   cross-language fixture plan and an on-chain bounds assertion.
7. Add duplicate-ID rejection, strict parser behavior, existing-identity
   non-migration, stale-doc cleanup, and the existing `IdentityKeyAdditionFlow`
   reuse decision to scope.
