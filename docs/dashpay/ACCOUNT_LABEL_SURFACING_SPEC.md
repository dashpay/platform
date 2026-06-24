# DashPay — `encryptedAccountLabel` receive-side surfacing (DIP-15 §8.5)

> **Status:** REVIEWED (5 lenses, 2026-06-24) — ready to implement.
> **Date:** 2026-06-24. **Branch:** `feat/dashpay-m1-sync-correctness`.
> **Closes:** the 🟡 receive-side follow-up of `DIP_CONFORMANCE_GAPS.md` §1.2 /
> `TODO.md` P1 ("the label is never decrypted/shown on receive — write-only").
> **Scope:** small, well-precedented. No new crypto, no FFI handle types, no
> contract change. Mirrors the existing `payment_channel_broken` machinery for
> *where it is set/persisted*, but is **direction-specific** (incoming-only), not
> a symmetric relationship property.

---

## 0. Review resolutions (2026-06-24, 5 independent lenses vs. the actual code)

Feasibility, simplicity, failure-modes, security, and DIP-15 domain-fit reviewers
each validated the spec against the code. The design is buildable (no feasibility
blockers) and right-sized. The must-fixes below are **folded into the body**:

- **B1 (failure-modes, blocker).** The rotation reset must live in
  `apply_rotated_incoming_request` (the in-place `get_mut` path rotation actually
  takes), **not only** `reestablish_preserving_metadata` (effectively unreachable
  for already-established contacts). Otherwise a stale label persists — and is
  re-persisted — against new key material. → §3.1.
- **Direction semantics (domain-fit + failure-modes, must-fix).** The label is
  **direction-specific**: our *outgoing* request carries a label *we* chose; the
  contact's label is on the *incoming* request only. Derive strictly from
  `incoming_request`, persist onto the **incoming FFI row only**, and restore from
  that row — do **not** copy the `alias`/`payment_channel_broken` "stamp both rows
  + first-non-null fold" pattern (those are genuinely symmetric; this is not). → §3.4.
- **Accept-path `shared` scope (failure-modes N3).** `shared` is computed *inside*
  `accept_register_external_validated` and not returned. Call the decrypt helper
  from inside that function (and from the drain Ok-branch) — never thread `shared`
  up the stack. → §3.3.
- **Lock discipline (failure-modes S2).** The helper is self-contained: it takes
  its own write guard, reads + decrypts (synchronous, no `.await`) + stores under
  it, exactly like `mark_contact_channel_broken`. Callers hold no guard across it.
  → §3.2.
- **Naming (feasibility).** `account_label` is already a send-side identifier in
  the same module (the owner's *outgoing* label). Name the receive-side field
  **`contact_account_label`** (Rust) / **`contactAccountLabel`** (Swift). → §3.1, §3.5.
- **Q-A resolved (feasibility + simplicity + code re-read).** The `EstablishedContact`
  is inserted at accept **step 3** (`add_sent_contact_request` auto-establish,
  `state/managed_identity/contact_requests.rs:103`) — *before* step 4's
  `accept_register_external_validated` (`network/contact_requests.rs:2292`); step 5
  (`:2322`) only re-reads it. So the `get_mut` helper finds the row in both accept
  branches. **No construction-time fallback needed.** (The domain-fit reviewer
  misread step-5 retrieval as insertion.)
- **Sanitize garbage (security LOW + Q-D).** AES-CBC has no integrity; a corrupt /
  non-conforming-sender ciphertext can PKCS7-unpad into valid-UTF-8 garbage.
  Coerce empty/whitespace **and control-char/non-printable** decrypts → `None` so
  the UI shows nothing rather than garbage. → §3.2.
- **Third Swift site (feasibility).** Restore is a round-trip: store-in
  (`PlatformWalletPersistenceHandler.swift:~1871`), **restore-out write-back**
  (`~4800`, `row.alias = …`), and the SwiftData column. All three need the field. → §3.5.

Confirmed safe / no change: consensus enforces `maxItems:80` so no length-DoS
(`dashpay.schema.json:229`); SwiftUI renders `String` *variables* verbatim (no
format-string injection); `decrypt_account_label`'s `try_into().unwrap()`
(`account_label.rs:104`) is unreachable (guarded by the `<16` check at `:99`); the
plaintext-at-rest delta is negligible (the ciphertext is already persisted and the
device holds the keys). Backfill (Q-B) correctly deferred — DashPay is unreleased.

---

## 1. Problem

DIP-15 §8.5 defines an optional `encryptedAccountLabel` on a `contactRequest`: a
human-readable label the **sender** attaches to the receiving account they are
sharing ("clients should display this … when sending payments" — it is a
**payment-routing hint**, not a decorative nickname), encrypted with the **same
ECDH shared key** as `encryptedPublicKey` (different prepended IV;
`IV(16) ‖ AES-256-CBC(32–64)` = 48–80 bytes).

The send side is conformant (`platform_encryption::encrypt_account_label`
normalizes length). The **receive side captures the ciphertext but never decrypts
or shows it**:

- Ingest stores the raw bytes on the incoming request
  (`network/contact_requests.rs:2544` → `ContactRequest.encrypted_account_label:
  Option<Vec<u8>>`, `types/dashpay/contact_request.rs:29`).
- They round-trip to SwiftData as `PersistentDashpayContactRequest
  .encryptedAccountLabel: Data?` (already persisted,
  `PersistentDashpayContactRequest.swift:83`) — but nothing ever calls
  `platform_encryption::decrypt_account_label`.
- The field is effectively **write-only**. The user never sees the label their
  contact chose for the account they pay into. Last partial item on DIP-15 §8.5.

## 2. Key facts (code-verified)

1. **The decrypt primitive exists**, bidirectional + padding-stripping:
   `platform_encryption::decrypt_account_label(&[u8;32] shared_key, &[u8]
   ciphertext) -> Result<String, CryptoError>`
   (`rs-platform-encryption/src/account_label.rs:95`; strips trailing spaces at
   `:109`). **No crypto work.**

2. **The label uses the same ECDH `shared` key as the xpub** (DIP-15 §8.5;
   dashj/android `encryptExtendedPublicKey` returns both ciphertexts from one
   `aesKey`, `research/06:166`). So decryption is only possible where that
   `shared` is materialized — and in the seedless architecture that is **exactly
   two signer-bearing sites**, both of which compute `shared` then call
   `register_external_contact_account`:
   - **Drain**, `RegisterExternal` arm — `shared` at
     `network/contact_requests.rs:1808`, register at `:1819`, `Ok(())` at `:1828`.
   - **Accept**, `accept_register_external_validated` — `shared` at `:2411`,
     register at `:2417`.
   The unattended recurring sweep has **no signer** (`:1457`) and cannot decrypt,
   so a "reconcile-only" approach (à la `reconcile_dashpay_rescan`) is impossible.

3. **The label is direction-specific.** `EstablishedContact` holds both
   `outgoing_request` and `incoming_request` (`established_contact.rs:19,22`),
   each with its own `encrypted_account_label`. The **contact's** label is on
   `incoming_request`; our outgoing request carries a label *we* sent (today
   always `None`, but the send API accepts one — `send_contact_request_with_
   external_signer(account_label: Option<String>)`, `:374`). Decrypt **only**
   `incoming_request.encrypted_account_label`.

4. **`payment_channel_broken` is the template for where-it's-set/persisted** (a
   derived field on `EstablishedContact` set during the account build, persisted
   via the `established` changeset by `mark_contact_channel_broken`, `:2131`).
   `contact_account_label` mirrors it for transport, **but is incoming-only**, not
   stamped onto both rows (fact 3).

5. **`collect_account_build_candidates` skips contacts whose external account is
   already built** (`has_external`, `:1407`). So the drain's `RegisterExternal`
   runs **once per new (or rotated) contact**: rotation tears down the stale
   external account (`sync_contact_requests` removes `dashpay_external_accounts`
   for rotated contacts, `:1152`), flipping `has_external` false so the build
   re-runs. A contact established *before this feature ships* will not re-drain →
   §3 Q-B (backfill, deferred).

6. **DashPay has never shipped** (PR #3841 unmerged). "Pre-feature" contacts live
   only on dev/test devices.

7. **Swift-SDK architecture rule** (`packages/swift-sdk/CLAUDE.md`): Swift
   persists/loads/bridges only; **all decryption stays in Rust**. This design
   complies — Rust decrypts; Swift stores + renders the plaintext string.

## 3. Chosen approach

Decrypt the contact's label **in Rust, inside the two signer-bearing register
sites** (drain + accept), via one self-contained helper that derives strictly
from `incoming_request`, stores the plaintext on
`EstablishedContact.contact_account_label: Option<String>`, and surfaces it
through an **incoming-row-only** projection.

### 3.1 Data model (Rust)

Add to `EstablishedContact`
(`packages/rs-platform-wallet/src/wallet/identity/types/dashpay/established_contact.rs`):

```rust
/// The contact's decrypted DIP-15 `encryptedAccountLabel` — a human-
/// readable label the contact attached to the receiving account they
/// shared (a payment-routing hint, e.g. "Main wallet"). Derived during
/// the external-account build by decrypting **`incoming_request`**'s
/// `encrypted_account_label` with the ECDH shared key (never the
/// outgoing request — that is our own label). `None` if the contact
/// sent no label or it could not be decrypted to printable text.
/// Cosmetic: a decrypt failure never breaks the payment channel.
#[cfg_attr(feature = "serde", serde(default))]
pub contact_account_label: Option<String>,
```

Reset rules (so the field never goes stale):
- `EstablishedContact::new` → `None`.
- `reestablish_preserving_metadata` → `None` (re-derived from the fresh
  `incoming_request`; for completeness, though it is not the rotation path).
- **`apply_rotated_incoming_request`** (`state/managed_identity/contact_requests.rs`,
  the in-place rotation mutation that swaps `incoming_request` and clears
  `payment_channel_broken`): **clear `contact_account_label = None`** there too.
  **This is the load-bearing reset (B1)** — rotation goes through this `get_mut`
  path, not the constructor; without it the old label persists against new keys.

### 3.2 Decrypt + persist helper (Rust, platform-wallet)

Self-contained, modeled on `mark_contact_channel_broken` (`:2131`):

```rust
/// Decrypt the contact's incoming `encryptedAccountLabel` with the ECDH
/// shared key and store the printable plaintext on the established
/// contact. Best-effort + cosmetic: no label / decrypt failure / garbage
/// → leaves or sets `None`; it MUST NOT break the channel or fail the
/// caller. Persists via an `established` changeset entry (same path as
/// the broken-channel flag). Self-contained locking: takes its own write
/// guard; the decrypt is synchronous (no `.await`, no re-lock).
async fn store_contact_account_label(
    &self,
    identity_id: &Identifier,
    contact_id: &Identifier,
    shared_key: &[u8; 32],
)
```

Body (single write guard, mirroring `mark_contact_channel_broken`):
1. `write().await`; `get_wallet_info_mut` → `managed_identity_mut` →
   `established_contacts.get_mut(contact_id)` (present by fact §2 — drain runs for
   established contacts; accept establishes at step 3 before this runs).
2. Read `contact.incoming_request.encrypted_account_label`:
   - `None` → return (leave field; no changeset).
   - `Some(ct)` → `platform_encryption::decrypt_account_label(shared_key, ct)`:
     - `Err(_)` → `tracing::debug!` and return. **No** `mark_contact_channel_broken`.
     - `Ok(s)` → **sanitize**: `let s = s.trim();` if `s.is_empty()` or `s` contains
       a control char (`s.chars().any(char::is_control)`) → treat as `None`
       (suppresses CBC-no-integrity garbage; security LOW + Q-D). Else
       `Some(s.to_string())`.
3. **Skip if unchanged** (`if contact.contact_account_label == new { return }`,
   exactly like `mark_contact_channel_broken:2142`) — no churn changeset.
4. Set the field; `snapshot = contact.clone()`; `cs.established.insert(
   SentContactRequestKey{owner_id, recipient_id: contact}, snapshot)`;
   `self.persister.store(cs.into())` (synchronous, under the guard).

### 3.3 Call sites (both pass `shared`; neither holds a guard across the helper)

- **Drain `RegisterExternal` Ok-branch** (`:1828`, after
  `register_external_contact_account` succeeds): `shared` is in scope (`:1808`).
  `self.store_contact_account_label(&entry.owner_identity_id, &entry.contact_id,
  &shared).await`.
- **Accept** — inside `accept_register_external_validated`, after the `:2417`
  register succeeds (where `shared` from `:2411` is in scope; it is **not**
  returned to the caller, so the call must be here): `self.
  store_contact_account_label(our_identity_id, contact_id, &shared).await`.

No change to `PendingContactCryptoOp::RegisterExternal`, `AccountBuildCandidate`,
`register_external_contact_account`'s signature, or the persisted queue schema —
the helper re-reads `incoming_request` itself (fact §2.2; the op deliberately
carries no label, `:1651`).

### 3.4 FFI transport — incoming-row-only (rs-platform-wallet-ffi)

`EstablishedContact` projects to **two** `ContactRequestFFI` rows (outgoing +
incoming). Because the label is direction-specific (fact §2.3):

- Add `contact_account_label: *const c_char` to `ContactRequestFFI`
  (`contact_persistence.rs`; allocate/free as the existing `alias` C string does).
- Populate it **only on the incoming projection** (`from_established_incoming` /
  the `is_outgoing == false` row) from `EstablishedContact.contact_account_label`.
  Leave it **null on the outgoing row**. Do **not** stamp both rows.
- Restore fold (`persistence.rs:~3996`, `PairAccumulator`): take
  `contact_account_label` **from the incoming row specifically** (the
  `is_outgoing == false` member), not OR / first-non-null. Assign it in the
  `EstablishedContact` reconstruction at `:~4010` (after `::new`, like the other
  manual field assigns).
- Free the C string in the `ContactRequestFFI` free path.

### 3.5 SwiftData + UI

- `PersistentDashpayContactRequest`: add `contactAccountLabel: String?` (additive,
  default `nil` → lightweight migration), documented as **system-derived,
  read-only, the contact's label for their account** — distinct from the
  owner-private `contactAlias`/`contactNote`. Only the incoming-direction row
  carries it.
- `PlatformWalletPersistenceHandler` — **three** sites (feasibility): (i) the
  SwiftData column; (ii) store-in on upsert (`~1871`, decode FFI → assign); (iii)
  restore-**out** write-back (`~4800`, `row.contact_account_label =
  duplicateCString(...)`). The §7 round-trip test exercises (iii).
- `ContactDetailView`: render read-only when present — a labeled row captioned
  toward the DIP's payment-routing intent (e.g. "Their account" /
  `Label(contactAccountLabel, systemImage: "wallet.pass")`), visibly **not** the
  owner's private alias/note. `ContactsView` may show it as a subtitle (optional).
- No Swift decryption; no FFI handle getter (the `@Query`-row path is the
  surfacing channel).

## 4. Alternatives rejected

- **Decrypt lazily in a `reconcile_*` step of `dashpay_sync`.** Impossible: the
  recurring sync has no signer, cannot compute `shared` (fact §2.2).
- **Carry the label ciphertext in `PendingContactCryptoOp::RegisterExternal`** /
  `AccountBuildCandidate`. Unnecessary: the helper re-reads `incoming_request`
  itself; threading it changes the persisted queue schema for no benefit (Rule 2).
- **A dedicated `DecryptAccountLabel` pending-crypto op.** More machinery (variant,
  drain arm, re-enqueue/clear discipline, the "permanent >0" footgun) for a value
  that already rides the `RegisterExternal`/accept flow.
- **Treat it as a symmetric relationship property (stamp both rows, first-non-null
  fold) like `alias`.** Semantically wrong (fact §2.3) — could surface our own
  outgoing label as the contact's. Hence the incoming-only projection (§3.4).
- **Decrypt in Swift** from the persisted ciphertext. Violates the swift-sdk
  no-crypto rule; needs the shared key across FFI.
- **A new FFI handle getter** (`established_contact_*`). Redundant — the `@Query`
  row path already carries the value to the UI.

## 5. Open decisions — resolved

- **Q-A (accept ordering).** RESOLVED: established at step 3, before the helper
  (§0). Use the `get_mut` helper; no construction fallback.
- **Q-B (backfill).** RESOLVED: **accept it** — pre-feature established contacts
  (dev devices only, fact §2.6) keep `None` until wipe + re-establish; zero
  production impact (DashPay unreleased). Note in `TODO.md`; no backfill code.
- **Q-C (UI wording).** RESOLVED: caption toward the DIP-15 payment-routing
  meaning ("Their account") — not "Account label" — and visibly distinct from the
  owner's alias/note. No reference-client display convention exists (we're first,
  like `contactInfo`).
- **Q-D (empty/garbage).** RESOLVED: sanitize in the helper — coerce empty,
  whitespace-only, **and control-char/non-printable** strings → `None` (§3.2).
  Lossless: the DIP padding convention can't distinguish a deliberate-spaces label
  from padding anyway (`account_label.rs:84`).

## 6. Failure modes

| Mode | Handling |
|---|---|
| Sender sent no label | `contact_account_label` stays `None`; no row rendered. |
| Decrypt fails (wrong key / corrupt / non-UTF-8) | Logged; set/left `None`; **channel not broken**; registration still `Ok` (the `:1828` Ok-arm is independent of the helper). |
| Garbled-but-valid-UTF-8 decrypt (CBC has no integrity) | Control-char/non-printable sanitize → `None` (§3.2). Cosmetic at worst. |
| Rotation | `apply_rotated_incoming_request` pre-clears to `None` (B1); the torn-down external account re-builds and the drain re-derives from the new incoming label. "Leave unchanged on failure" is safe **because** of this pre-clear. |
| Pre-feature established contact (dev only) | `None` until wipe+re-establish (Q-B). No production users. |
| Concurrency | Helper takes the write guard only; decrypt is synchronous (no `.await`/re-lock); idempotent skip-if-unchanged — no deadlock, no churn (mirrors `mark_contact_channel_broken`). |
| Length-DoS / injection | Consensus `maxItems:80` bounds the ciphertext; SwiftUI renders the variable verbatim. No finding. |

## 7. Test / verification plan

**Rust unit (platform-wallet, `cfg(test)` `SeedCryptoProvider`):**
- `contact_account_label_decrypted_on_external_register` — established contact whose
  **incoming** request carries `encrypt_account_label(shared, iv, "Main wallet")`;
  run the drain `RegisterExternal`; assert
  `established_contacts[contact].contact_account_label == Some("Main wallet")`.
  **Red before** wiring, **green after**.
- `contact_account_label_none_when_no_label` — incoming label `None` → stays `None`.
- `contact_account_label_decrypt_failure_is_cosmetic` — garbage ciphertext →
  registration `Ok`, label `None`, `payment_channel_broken == false`.
- `contact_account_label_control_chars_coerced_to_none` (Q-D/security) — a decrypt
  yielding control chars / empty → `None`.
- `rotation_resets_contact_account_label` (B1) — set a label, drive
  `apply_rotated_incoming_request`, assert reset to `None` (and re-derives on the
  next build).
- `outgoing_label_never_wins` (direction, §2.3/§3.4) — an established pair whose
  **outgoing** row carries a *different* raw `encrypted_account_label`; assert the
  surfaced `contact_account_label` is the **incoming** value, and an outgoing-only
  label yields `None`.

**FFI (rs-platform-wallet-ffi):**
- `incoming_row_carries_contact_account_label_outgoing_null` — mirror
  `established_rows_carry_payment_channel_broken_flag` (`contact_persistence.rs:589`):
  incoming projection carries the plaintext, outgoing projection is null, pending
  rows null.
- Persist→restore round-trip: `contact_account_label` survives
  encode→FFI→decode→restore-out (covers the third Swift site conceptually at the
  FFI layer).

**Swift:** `build_ios.sh` green (additive SwiftData migration compiles); a handler
unit assertion mapping `ContactRequestFFI.contact_account_label →
contactAccountLabel` on the incoming row. UI render verified by sim build.

**On-device / interop (rides funded devnet — deferred, consistent with the rest of
the suite):** two-simulator flow — Alice sends Bob a contact request **with** an
account label; Bob establishes and `ContactDetailView` shows the decrypted label.
The `SeedCryptoProvider` unit tests prove the crypto+storage path headlessly now.

## 8. Task breakdown

- **A — Rust core.** `contact_account_label` field + the three resets (esp. B1 in
  `apply_rotated_incoming_request`); `store_contact_account_label` helper
  (incoming-only, sanitize, self-contained lock); wire into the drain Ok-branch and
  inside `accept_register_external_validated`. Unit tests (red→green), incl.
  `outgoing_label_never_wins` and `rotation_resets_*`.
- **B — FFI transport.** `ContactRequestFFI.contact_account_label` (incoming row
  only; restore fold reads the incoming row; free path); FFI test.
- **C — Swift.** `PersistentDashpayContactRequest.contactAccountLabel`; handler
  store-in + restore-out (both sites); `ContactDetailView` read-only render (Q-C);
  `build_ios.sh`.
- **D — Docs.** Flip `DIP_CONFORMANCE_GAPS.md` §1.2 / `TODO.md` P1 receive-side
  follow-up to done; note Q-B(accept) backfill scoping.
