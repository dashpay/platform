# Spec — Imported identity cannot sign: emit full key-derivation breadcrumbs on discovery

Status: DRAFT v2 (reviewed by 4 lenses incl. blockchain-security; must-fixes folded in)
Scope: `packages/rs-platform-wallet` (discovery) + 1 optional Swift defense-in-depth check.
Related: `docs/dashpay/TODO.md` → "BUG (UAT 2026-06-19): an IMPORTED identity cannot sign".

## 1. Problem

An identity **rediscovered from a mnemonic** (gap-limit "discover identities" scan /
re-import) cannot **sign state transitions**. Signed ops fail with:

```
SDK error: Protocol error: Generic Error: No PersistentPublicKey row matches the supplied public-key bytes
```

A **create-in-app** identity signs fine. On-device the on-chain public keys *are*
persisted correctly; what is missing is the signing key's **private** material.

Concretely this restores **identity authentication-key signing** — register DPNS name,
set DashPay profile, and identity-credit transfers. It does **not** by itself fix
DashPay-contact-request ECDH (see §3 non-goals).

## 2. Root cause

Signing reads a **pre-stored** private key; an imported identity never materializes the
private key for its non-master keys.

- The Swift persist handler `persistIdentities` → `deriveAndStoreIdentityKey`
  (`PlatformWalletPersistenceHandler.swift:1584`) is the single key-materialization
  path. Per persisted key it branches on the **breadcrumb** `entry.derivationIndices`:
  present → re-derive the 32-byte scalar from `(seed, path)` via
  `key_wallet_derive_private_key_from_seed` at
  `getIdentityAuthenticationPath(identityIndex, keyIndex)` and store in the Keychain
  (**no private bytes cross the FFI** — Swift re-derives from the mnemonic); absent →
  `privateKeyKeychainIdentifier = nil` (watch-only).
- `KeychainSigner` (identity path, `key_type < 5`) signs from the **pre-stored**
  Keychain bytes; it does not re-derive on demand.

Breadcrumb coverage differs:

- **Create** (`registration.rs:297-304`): breadcrumb for **every** key (`key_index =
  key_id`) → all materialized → all signable.
- **Discovery** (`discovery.rs:281-285`): breadcrumb for **only the MASTER key**;
  the other keys arrive via `add_identity` → `keys_snapshot_changeset`
  (`identity_ops.rs:51-71`) with `derivation_indices: None` (watch-only). The selected
  signing key is a non-master HIGH/CRITICAL auth key → no private key → the misleading
  "No PersistentPublicKey row matches" (row exists; private key does not).

### Not the cause: the pasta-bridge derivation path

Bridge keys (`github.com/PastaPastaPasta/dash-bridge`, `src/crypto/hd.ts`) are derived
at `m/9'/{coin}'/5'/0'/{keyType}'/{identityIndex}'/{keyIndex}'`, registered with
`keyId == key.id == keyIndex` — **byte-for-byte identical** to platform-wallet's
`identity_auth_derivation_path` (key-wallet `dip9.rs`: 9 / testnet-coin 1 / identities
5 / subfeature 0 / ECDSA 0) and to create-in-app's `key_index = key_id`. The keys are
re-derivable from the mnemonic; the bridge is standard-compliant. The bug is purely the
discovery breadcrumb gap.

## 3. Goal / non-goals

Goal: a discovered identity whose keys are re-derivable from the wallet mnemonic at the
DIP-9 auth path can sign with its authentication keys, exactly like create-in-app — by
emitting a derivation breadcrumb for every such key during discovery so the existing
Swift handler materializes its private key.

**Seed-availability precondition (load-bearing).** A breadcrumb is a *claim the client
may be unable to honor*. Swift materializes a key only if the wallet's mnemonic is
retrievable from the Keychain (`retrieveMnemonicUTF8Bytes(walletId)`); otherwise
`deriveAndStoreIdentityKey` returns `nil` and the row stays watch-only (no half-state,
no regression). So this fix makes keys signable **iff the wallet's seed is on-device**.
For a normal mnemonic import that holds; for a true seed-less watch-only wallet the
breadcrumb is correctly inert.

Non-goals:
- No FFI/Swift behavioral change to the materialization path (it already works); the one
  optional Swift change is a defensive re-verify (§7.2).
- No on-demand re-derivation in `KeychainSigner` (larger refactor; unnecessary once
  breadcrumbs are complete).
- **DashPay contact-request / contactInfo ECDH is out of scope.**
  `send_contact_request_with_external_signer` derives the ECDH private key + DashPay
  xpub directly from the **resident** in-process `Wallet`
  (`contact_requests.rs` CAVEAT: "watch-only wallets — no seed Rust-side — WILL fail at
  this step"). That is a separate seed-residency concern; this spec restores **auth-key
  signing** only. Whether contact requests already work for the app's import flow
  depends on whether that wallet is resident-key vs external-signable — to be confirmed
  during on-device verification (§8), tracked separately if still broken.
- Keys NOT re-derivable from this wallet's mnemonic at the DIP-9 auth path (BLS/EdDSA,
  foreign/rotated/imported keys) — stay watch-only (correct).

## 4. Chosen approach

In `discover_inner` (`discovery.rs`), replace the master-only breadcrumb emission
(`discovery.rs:277-286`) with a per-key verify-and-emit pass. Key shape (the master key
becomes the `key_id == 0` case of the loop; its dedicated `add_key` is removed):

**(a) Derive + verify candidates BEFORE taking the write lock** (resolves the borrow
conflict: candidate derivation needs `&Wallet`/`&master`, breadcrumb emission needs
`&mut info` — they cannot co-borrow the manager guard). After the identity is fetched
(network, no lock held):

For each `(key_id, on_chain_key)` in `identity.public_keys()`:
1. Derive the candidate ECDSA auth **keypair** at `(identity_index, key_index = key_id)`
   from the same `KeyHashSource` as the master probe:
   - `KeyHashSource::Master(master)` → `derive_ecdsa_identity_auth_keypair_from_master`
     (lock-free; yields `private_key: Zeroizing<[u8;32]>`).
   - `KeyHashSource::ResidentWallet` → re-acquire a **read** lock on the manager, fetch
     `&Wallet`, call `derive_identity_auth_keypair` (yields `ExtendedPrivKey`; take
     `.private_key.secret_bytes()`), drop the read lock before the write lock below.
2. **Verify** with the canonical consensus primitive — do NOT hand-roll a per-type
   pubkey/hash compare:
   ```rust
   let reproduces = on_chain_key
       .validate_private_key_bytes(&candidate_private_scalar, network)
       .unwrap_or(false);
   ```
   `validate_private_key_bytes` (rs-dpp, `identity_public_key/v0/methods/mod.rs`) is the
   **same primitive the protocol uses to validate key ownership**, so the wallet's match
   is identical to consensus and cannot drift. For ECDSA it recomputes the **compressed**
   pubkey from the candidate scalar and compares; it also handles ECDSA_HASH160
   (ripemd160_sha256) and returns `false`/`Err` for BLS/EdDSA/unsupported — so a
   non-reproducible key is fail-safe.
   **Empirically verified (TDD) + corrected by the security re-review:** `validate_private_key_bytes`
   is *compressed-only* for ECDSA (it does NOT match a 65-byte uncompressed on-chain key),
   and contrary to an earlier draft of this spec, Platform does **NOT** reject uncompressed
   identity keys at registration — `UncompressedPublicKeyNotAllowedError` lives only in the
   asset-lock signing path, and identity proof-of-possession accepts uncompressed keys.
   Compressed-only matching is nonetheless **correct**: the wallet only ever *derives* the
   33-byte compressed form, so an uncompressed externally-registered key is simply not
   wallet-derivable and correctly stays watch-only (graceful degradation, never a wrong-key
   hazard). No code change needed — just don't justify it with the false "rejected at
   registration" claim.
3. Record the decision: `key_id → Some((wallet_id, identity_index, key_id))` if
   `reproduces`, else `None` (watch-only). Zeroize the scalar.
   If an **ECDSA** auth key fails to verify at its `key_id` candidate, log at
   `warn!` (not `debug`) with `key_id` (no key material) so a still-unsignable import is
   field-diagnosable.

**(b) Emit one batched changeset under the write lock.** Take the write lock once;
`add_identity` (as today) then a **single** `IdentityKeysChangeSet` carrying every key
with its decided breadcrumb-or-`None` (new `ManagedIdentity::add_keys(Vec<(IdentityPublicKey,
Option<breadcrumb>)>)` helper, or inline). This replaces the N separate `add_key`
round-trips, and removes the order-dependent "watch-only then override" merge (one
authoritative key changeset per identity). Document that this batch is the single
source of per-key breadcrumbs for a discovered identity.

**(c) Shared with the index-load path.** Steps (a)+(verify) are extracted into
`IdentityWallet::derive_key_breadcrumbs(identity, identity_index, network, master:
Option<&ExtendedPrivKey>)`, used by **both** `discover_inner` AND
`load_identity_by_index_inner` (`loading.rs`) — the latter had the identical master-only
bug (it backs the public `loadIdentity(atIndex:)` API). The resident-source `get_wallet`
lookup fails loud (consistent with every other manager lookup) rather than silently
leaving all keys watch-only.

The candidate-derivation cost is ≤ (#on-chain keys) secp256k1 derivations per discovered
identity, no network.

## 5. Alternatives considered / rejected

- **Assume `key_index = key_id`, emit unconditionally (no verify).** Would store a
  WRONG private key for any on-chain key not re-derived from this seed at that slot —
  not just hypothetical future creators, but any rotated / multisig / imported /
  foreign key. Swift would then sign with an unauthorized key → Drive rejects at best.
  Rejected; verify-before-emit is load-bearing.
- **Hand-rolled per-type pubkey match.** Rejected — it can drift from consensus
  per-type semantics, whereas `validate_private_key_bytes` IS the consensus primitive.
  (The review feared a hand-rolled compressed-only match would miss uncompressed keys;
  that turned out moot — uncompressed ECDSA keys are protocol-disallowed — but using the
  canonical primitive is still the right call for drift-resistance.)
- **Per-key gap scan (`key_index` 0..N per on-chain key).** O(keys×gap) for zero present
  benefit (`key_id == key_index` holds for bridge + app). Possible future fallback;
  not now.
- **On-demand re-derivation in `KeychainSigner`.** Larger surface; breadcrumb
  completeness is its prerequisite anyway, so this spec is a strict subset. Deferred.
- **Pass private bytes across the FFI.** Violates the swift-sdk no-secret-transit rule.
  Rejected.

## 6. Failure modes & edge cases

- **Non-ECDSA on-chain key** (BLS/EdDSA): `validate_private_key_bytes` returns
  false/err → watch-only. Correct.
- **Disabled key** (`disabled_at` set): **emit** the breadcrumb (matches create /
  `registration.rs`, which doesn't special-case; the key is never selected for signing —
  every signer uses `allow_disabled = false` — so materializing it is inert). Decision
  closed: emit.
- **`key_index != key_id`** (non-conforming creator): candidate won't verify →
  watch-only → that key stays unsignable (no regression), `warn!`-logged.
- **`ECDSA_HASH160` auth key**: covered by `validate_private_key_bytes`.
- **Key rotation / keys added after first discovery**: a fresh **full** rescan re-reads
  `identity.public_keys()` and verify-emits each → rotated-in keys are picked up
  (provided `key_index == key_id`). No separate rotation handling.
- **ENCRYPTION (ECDH) key**: if it's an ECDSA key at `key_id`, it verifies and gets a
  (harmless) breadcrumb. DashPay ECDH does **not** consume the materialized scalar — it
  derives from the resident seed separately (`contact_info.rs`) — so this fix neither
  fixes nor breaks ECDH (which remains seed-residency-bound, §3 non-goal).
- **Partial failure** (per-key granularity): with the batched changeset (§4b) the keys
  land atomically per identity; a persist failure leaves the whole batch unpersisted and
  is retried on the next full rescan. (If kept as per-key calls instead, some keys could
  be breadcrumbed and others not within one identity — another reason to batch.)
- **Resident vs master source parity**: candidate derivation uses the same source as the
  master probe, so an external-signable wallet derives from the resolved xpriv.
- **Observed (out-of-wallet) identities**: never enter `discover_inner` (added via
  `add_out_of_wallet_identity`) → no breadcrumbs → watch-only. Correct.

## 7. Security considerations

1. **Wrong-key signing** — primary risk, eliminated by verify-before-emit using the
   canonical `validate_private_key_bytes` (§4 step 2). No path emits a breadcrumb
   without a successful match; preimage resistance precludes a false-positive.
2. **(Optional, recommended) Swift cross-FFI mirror check** — the Rust verify and the
   Swift re-derivation are different code on opposite sides of the FFI. Add to
   `deriveAndStoreIdentityKey`: after deriving the scalar, compute
   `ripemd160(sha256(pubkey))` and compare to the already-passed `entry.publicKeyHash`;
   on mismatch, log + return `nil` (watch-only) instead of storing. Turns a future
   cross-side derivation drift into a loud, fail-safe miss. Cheap; small Swift change.
3. **Confused-deputy / cross-wallet** — the breadcrumb carries the scanning wallet's id;
   the verify gate requires that wallet's seed to actually reproduce the key, so a
   foreign identity sharing a `key_id` cannot bind (its candidate won't verify). Sound.
4. **identity_index correctness** — all keys of one identity share the scan-cursor
   `identity_index`; `key_index = key_id` per key. A wrong index fails safe (no verify).
5. **Private-key-at-rest** — materializes the non-master auth scalars into the Keychain,
   the **same exposure create-in-app already has**; no new secret class, no bytes over
   the FFI. **No secret logging**: the candidate scalar stays in `Zeroizing`; logs carry
   only `key_id` / public hashes. (Implementation must not log `private_key`/`derived`.)
6. **DoS / cost** — bounded (≤ #keys derivations, no network).

## 8. Test / verification plan

Rust unit tests (`discovery.rs` test module, mock SDK + planted identity,
`RecordingPersister` capturing changesets):
- `discovery_emits_breadcrumb_for_every_reproducible_key`: N keys derivable at
  `key_index = key_id` → N breadcrumbed `IdentityKeyEntry`s
  (`derivation_indices == Some((identity_index, key_id))`).
- `discovery_leaves_non_reproducible_key_watch_only`: a planted key whose data does NOT
  reproduce at its `key_id` (foreign pubkey / non-ECDSA) → `derivation_indices == None`.
- `breadcrumb_decisions_matches_hash160_key`: an `ECDSA_HASH160` key verifies by hash and
  gets a breadcrumb (pins the second valid ECDSA representation). (Uncompressed ECDSA is
  NOT tested — protocol-disallowed at registration, so not a valid on-chain key.)
- `discovery_backfills_breadcrumbs_on_full_rescan`: re-running discovery from index 0
  over an already-present identity re-emits the breadcrumbs (idempotent upsert; pins the
  §9 migration claim).
- Master-key parity: the master key still gets its breadcrumb (now via the loop/batch).

On-device (testnet sim, simulator-control):
- Re-import the testnet mnemonic, run a **full rescan from index 0** (see §9), then
  perform an auth-signed op (set DashPay profile / register a name) as a bridge-created
  identity. Expected: succeeds (was "No PersistentPublicKey row matches"). Cross-check
  `privateKeyKeychainIdentifier` is set for the signing key.
- ECDH check (scopes §3 non-goal): attempt a DashPay **contact request** from the
  imported identity. If it fails with the seed-residency CAVEAT, file the ECDH item;
  if it succeeds, the app's import wallet is resident-key and ECDH is already covered.
- Regression: a create-in-app identity still signs (unchanged path).

## 9. Rollout / migration

Rust change in `discover_inner` (+ optional small Swift mirror check, §7.2). No
schema/FFI signature change.

**Backfill requires a FULL rescan from index 0**, not the default "Re-scan for
Identities" resume. `discover()` with `start_index = None` resumes at
`highest_registration_index + 1` (`discovery.rs:181-184`) and **skips** an
already-imported (broken) identity at index N — so the default re-scan does NOT heal it.
The migration must drive `start_index = Some(0)` (the FFI's
`start_index_or_neg1 = 0` cold-rescan path). Action items:
- Confirm which iOS control passes `0` (cold full rescan) vs `nil` (resume); document
  the exact user action (or add a "Full rescan" affordance) that backfills.
- Alternative for an affected user: delete + re-import the wallet (resets the bucket so
  index 0 is re-scanned).
Document the chosen migration action in the PR so users with an already-broken import
know how to heal it.
