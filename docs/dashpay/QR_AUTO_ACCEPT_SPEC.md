# DashPay QR Auto-Accept (DIP-15) — Implementation Spec

Decision (2026-06-24): build the DIP-15 `autoAcceptProof` QR flow, **faithful to the
DIP-15 wire formats** so we are a correct reference implementation. Research (incl. the
finding that no reference client implements this today, so it is iOS-first / convention-
setting) informed this spec. Invitations (DIP-13) are queued next.

> **Status:** IMPLEMENTED (2026-06-24) across Rust + FFI + Swift; `build_ios.sh` green,
> platform-wallet 299 + ffi 117 tests green. REVIEWED (4-lens: DIP-fidelity / security /
> feasibility / scope) and revised — §10. The first draft's §4 was materially wrong
> (verify can't use `&Wallet` in the seedless drain; the drain lacks the identity signer;
> the sweep parser drops the proof) — all fixed. **Owner decisions:** TTL = 1h fixed;
> auto-accept = always automatic; whole feature in one pass; DIP-literal HD-derived owner
> key (scoped raw-key export). **On-device:** My-QR UI + DPNS-name guard verified; the full
> QR-generate→scan→auto-accept loop is pending a DPNS-named *local* identity (the available
> devnet wallets have on-chain names not cached in `PersistentIdentity.dpnsName`). Follow-up
> (P3): resolve the owner's DPNS name on-chain in `build_auto_accept_qr` when the local
> field is empty.

## 1. Problem & goal

DashPay contact establishment is two manual taps. DIP-15 defines an optional
`autoAcceptProof` so a party can pre-authorize automatic acceptance — the canonical use
case is a **merchant / in-person QR**: show a QR, the scanner sends a contact request that
the owner's client auto-accepts with no manual tap. The proof crypto exists and is
unit-tested (`auto_accept.rs`) but is **dormant** — nothing generates, verifies, or acts
on it. Goal: wire the full three-role flow, end to end (Rust + FFI + Swift + on-device).

### Non-goals
- Not Invitations (DIP-13 `dashpay://invite` + AssetLock onboarding) — separate, queued next.
- No Android interop today (no reference client verifies the proof); iOS-first. We still
  follow DIP-15 byte layouts so a future client can interop.
- No new on-chain artifact beyond the already-defined optional `autoAcceptProof` field.
- No `di=` identity-id URI fallback in v1 (DIP uses `du`; require a DPNS name — §9).
- No TTL picker, no opt-in toggle (always automatic) in v1 (§9).

## 2. The DIP-15 model — three roles

1. **Owner (QR shower, "Bob").** Derives an auto-accept key at `m/9'/5'/16'/expiry'`,
   embeds the **private key + expiry** in a QR (`dash:?du=<bob_username>&dapk=<key_blob>`),
   shows it. (`expiry = now + 1h`.)
2. **Scanner ("Carol").** Scans, resolves `du`→Bob's identity, decodes `dapk`→(private key,
   expiry), derives her friendship `accountReference` to Bob, **signs `Carol.$ownerId ‖
   Bob.toUserId ‖ accountReference` with the handed key**, and sends a contactRequest to
   Bob carrying that proof.
3. **Owner receives + auto-accepts.** Bob's client (at a signer-present drain) verifies the
   proof against **his own** re-derived auto-accept **public** key and, if valid and
   unexpired, **auto-accepts** (sends the reciprocal) with no manual tap.

Why the scanner signs (not the owner): the signed message includes the scanner's
`$ownerId`, unknown at QR-create time — so the owner delegates signing via the (expiry-
bounded) private key. This per-sender binding means a leaked proof can't be replayed by a
*different* sender.

## 3. DIP-15 wire formats — normative-for-us

These are wire-faithful to DIP-15 (fidelity review: byte-for-byte match). Where DIP-15 is
silent, the value below is **normative for our implementation** — a future interop client
MUST match it or verification silently fails.

**Auto-accept key blob** (`dapk` value), 38 bytes for ECDSA:

| field | size | value |
|---|---|---|
| key type | 1 | `0x00` (ECDSA_SECP256K1) |
| timestamp/expiry (= derivation index) | 4 | u32, **big-endian** *(DIP-silent → normative)* |
| key size | 1 | `0x20` (32) |
| key | 32 | secp256k1 **private** key |

**Proof blob** (`autoAcceptProof` field), 70 bytes for ECDSA, 38–102 range:

| field | size | value |
|---|---|---|
| key type | 1 | `0x00` |
| key index (= expiry, same value as the blob) | 4 | u32, **big-endian** |
| signature size | 1 | `0x40` (64) |
| signature | 64 | compact ECDSA |

**Signed message** *(DIP names the fields; hashing/encoding DIP-silent → normative)*:
`SHA256($ownerId(32) ‖ toUserId(32) ‖ accountReference(4, little-endian))`, where
`$ownerId` = the contactRequest **sender (scanner)**, `toUserId` = the QR **owner**, and
`accountReference` is the **raw masked u32** the contactRequest carries (`version<<28 |
masked_index`). Matches the existing `auto_accept.rs::build_message_hash`. **Security pin
(§6):** the verifier MUST bind `$ownerId` to the **consensus-authenticated document owner
id** (`doc.owner_id()`), never a self-reported field.

**Derivation path**: `m / 9' / 5'(mainnet, else 1') / 16' / expiry'`, all hardened; `expiry`
≤ 2^31−1 (hardened-index bound, ~year 2038 — reject at encode time). Matches code.

**URI**: `dash:?du=<dpns-username>&dapk=<base58(key_blob)>` (contact-only). Matches the
DIP-15 example. No `di=` fallback in v1.

## 4. Seedless integration (the corrected crux)

Our wallets are `ExternalSignable` — no seed in Rust; key material is reachable only via
the Keychain resolver/provider. The background sweep is **signerless**. Both verify
(needs the owner's auto-accept key) and auto-accept (sends a signed state transition) need
key material, so **neither runs in the sweep** — they ride the deferred-crypto queue +
the signer-present drain. The first draft got the mechanics wrong; corrected:

### 4.1 Sweep (signerless) — read the proof, enqueue, bounded
- **FIX (feasibility #2):** `parse_contact_request_doc` must read
  `props.get("autoAcceptProof")` into the parsed `ContactRequest` (today it's hard-coded
  `None`, so the proof is dropped before the queue). Mirror the outgoing reader.
- After `add_incoming_contact_request`, if the request carries an `autoAcceptProof` that
  passes a cheap **structural pre-check** (length 38–102, key-type `0x00`), enqueue
  `PendingContactCryptoOp::AutoAccept { sender_id }` (dedup key `(owner, sender, AutoAccept)`).
- **DoS bound (security #4):** cap queued `AutoAccept` ops per owner (constant, e.g. 64);
  beyond the cap, skip enqueue (the request is still manually acceptable — nothing lost).
  Log the drop (no silent cap).

### 4.2 Drain (signer present) — needs BOTH signers
- **FIX (feasibility #3 / scope M1):** the drain needs the identity `Signer<IdentityPublicKey>`
  (to send the reciprocal) **and** the `ContactCryptoProvider`. Thread a `signer` into
  `drain_pending_contact_crypto` and add a `signer_handle` to the drain FFI (matching the
  send/accept FFIs). Existing arms ignore it (additive bound). **Note:** the drain FFI is
  the same one `unlockWalletFromKeychain` calls (needs-unlock work) — that call site now
  passes the Swift `KeychainSigner` too.
- Per `AutoAccept` entry, in order:
  1. **Local verify FIRST, before any network fetch** (security #4 — anti-DoS): build the
     path `m/9'/coin'/16'/expiry'` (expiry from the proof header), derive the owner's
     auto-accept **public** key via `provider.receiving_xpub(&path).public_key` (**FIX
     feasibility #4** — verify needs only the pubkey; no `&Wallet`), then
     `verify_auto_accept_proof_with_pubkey(pubkey, proof, sender_id = request.sender_id
     (= doc.owner_id), recipient_id = self_identity, account_ref = request.account_reference)`.
  2. **Expiry check** against the **same** timestamp that keyed verification
     (`now > expiry → reject`).
  3. If valid + unexpired → `accept_contact_request_with_external_signer(request, signer,
     provider)` (sends the reciprocal; idempotent — adopts if already reciprocated).
- **Verdict mapping (security #3):** invalid signature / wrong params / expired /
  out-of-range index (the `Err` from path derivation) ⇒ **permanent: clear the entry**
  (the request falls back to a normal manual-acceptable pending request). Signer/network
  unavailable ⇒ **transient: leave queued** for the next drain. Never `mark_channel_broken`
  (there's no channel yet).

Consequence: auto-accept completes at the owner's next signer-present moment (unlock or any
DashPay action), not instantly in the background. Consistent with the seedless model.

## 5. Interface / data flow per layer

### 5.1 Rust — `auto_accept.rs` (extend; keep existing tested fns)
- KEEP `derive_auto_accept_private_key(wallet, network, expiry)` (owner, QR-create).
- ADD `encode_auto_accept_key_blob(secret_key, expiry) -> Vec<u8>` /
  `decode_auto_accept_key_blob(&[u8]) -> Result<(SecretKey, u32)>` (38-byte `dapk`).
- ADD `sign_auto_accept_proof(secret_key, scanner_id, owner_id, account_ref, expiry) -> Vec<u8>`
  — scanner signs with the **handed** key. Message bytes = `scanner_id ‖ owner_id ‖
  account_ref(LE)` (the existing `build_message_hash`); a doc-comment ties the param names
  to DIP roles (**scope M2** — the current `generate` models the owner as `sender_id`,
  the opposite; don't invert at wiring).
- ADD `verify_auto_accept_proof_with_pubkey(pubkey, proof, scanner_id, owner_id, account_ref) -> bool`
  — pure, no wallet (the drain's verify path).
- ADD `auto_accept_proof_expiry(proof) -> Option<u32>` and fold the expiry check into the
  acceptance entry point — **do not** expose a public bare `verify` that returns `true`
  for an expired proof (**security #2** foot-gun). Keep `verify_auto_accept_proof(wallet,…)`
  for owner-side tests only.
- ADD a URI codec `encode_dashpay_contact_uri(username, key_blob)` /
  `parse_dashpay_contact_uri(&str) -> Result<(username, key_blob)>` (pure, testable).
- Refactor `generate_auto_accept_proof` to `derive + sign` (test/convenience).
- Remove the stale `// TODO: Where and how we use these helpers?` and fix the docstring
  that references a now-real `verify_auto_accept_proof_with_pubkey` (**scope N3**).

### 5.2 Rust — changeset.rs + contact_requests.rs (flow)
- `PendingContactCryptoOp::AutoAccept { sender_id }` + `PendingContactCryptoKind::AutoAccept`
  — 9 sites (feasibility #1 change-list): enum, kind, `kind()`, storage `KIND_LABELS`,
  `kind_db_label`, the `kind_labels_match_enum` test, the drain's exhaustive `match`, and
  `count_account_build_ops` (decide inclusion — **yes**, so the needs-unlock banner counts
  pending auto-accepts; reword the banner copy, **scope S3**).
- `parse_contact_request_doc` reads `autoAcceptProof` (§4.1).
- `sync_contact_requests` enqueues `AutoAccept` (bounded) when a proof is present.
- `drain_pending_contact_crypto` gains the `signer` param + the `AutoAccept` arm (§4.2).
- Scanner send reuses `send_contact_request_with_external_signer(..., auto_accept_proof)`
  (already threaded). **scope M3:** the scanner must derive its `accountReference` first
  (in-signer, masked over the friendship xpub) and sign the proof over that **exact** value
  before broadcast — test that the signed `accountReference` equals the document's.
- DPNS resolve: `IdentityWallet::resolve_name(&str) -> Option<Identifier>` (feasibility #5;
  not `search_names`).

### 5.3 FFI (rs-platform-wallet-ffi)
- `platform_wallet_build_auto_accept_qr(wallet, identity_id, out_uri…)` — owner: resolve
  the wallet's DPNS name (error if none), `expiry = now + 3600`, derive the key, build the
  `dash:?du=…&dapk=…` URI; return it. (Single Rust entry — no decisions in Swift.) `now` is
  passed in from Swift (FFI can't read the clock deterministically) or read via a host hook.
- `platform_wallet_send_contact_request_from_qr(wallet, signer, core_signer, uri, out…)` —
  scanner: parse URI → resolve `du` → decode `dapk` → (derive accountRef, sign proof) → send
  the contactRequest with the proof. One call.
- `platform_wallet_drain_pending_contact_crypto` gains `signer_handle: *mut SignerHandle`
  (the identity signer) alongside the existing `core_signer_handle` (§4.2).

### 5.4 Swift (SwiftExampleApp)
- **My QR** (net-new, `DashPayProfileView`): a "Show my QR" affordance rendering the URI
  from `build_auto_accept_qr` via the existing `generateQRCode` helper.
- **Scan** (net-new entry in the DashPay tab toolbar): present `QRScannerView`; add a new
  parse branch + result type (`ScannedContact{username, keyBlob}`) to `QRPayloadParser`
  (the existing `ScannedPayment` path doesn't fit a no-address URI — **scope N2**), route to
  `platform_wallet_send_contact_request_from_qr`.
- **Drain call-site:** `unlockWalletFromKeychain` now passes the `KeychainSigner` to the
  drain FFI (the added `signer_handle`).
- **Feedback (scope S4):** the auto-accepted contact lands in `ContactsView` via `@Query`;
  add a light signal (the needs-unlock banner already counts the pending `AutoAccept`, so it
  shows "1 contact waiting…" until the drain completes, then it appears as a contact).

## 6. Security (4 must-fixes folded)

1. **Consensus-authenticated sender binding (must-fix #1):** verify binds `$ownerId =
   doc.owner_id()`. A malicious holder of a leaked QR key *can* sign a proof naming any
   sender, but cannot broadcast a contactRequest *as* a victim — platform consensus
   requires the doc to be signed by the owner's identity key. The verify gate MUST use the
   document owner id, never a proof-internal/client value.
2. **No expired-but-valid foot-gun (must-fix #2):** the only acceptance entry checks expiry
   against the same timestamp that keyed verification; no public bare `verify` returns
   `true` for an expired proof.
3. **Drain verdict mapping (must-fix #3):** invalid/expired/bad-index → permanent-clear;
   signer/network → transient-leave (§4.2). Prevents forever-churn.
4. **Queue bound + verify-before-fetch (must-fix #4):** cap `AutoAccept` per owner; run the
   local ECDSA verify + expiry before any `Identity::fetch`, so a spam-N-identities attacker
   can't turn the owner's unlock into O(N) network round-trips.
- **Private key in QR:** intrinsic to DIP-15 (the scanner must sign; the owner can't
  pre-sign without the scanner's id). Scoped (only auto-accept, not payments/identity),
  expiry-bounded (**1h**), blast radius = unwanted contact spam (removable via ignore).
  Acceptable documented trade-off, tightened by the short TTL (no off-switch since
  auto-accept is always-on, so the short TTL is the mitigation).
- **Replay:** signed message binds `(sender, owner, accountReference)`; the doc's unique
  index is `($ownerId, toUserId, accountReference)` — no cross-sender replay, no on-platform
  dup. Cross-network separated by coin-type in the path.

## 7. Failure modes
- **Signer absent when a proof arrives:** enqueued (bounded), completes on next drain;
  surfaced by the needs-unlock banner.
- **Expired / invalid / forged proof:** verify-gate rejects, entry cleared (permanent);
  request remains manually acceptable.
- **`du` resolves to wrong/missing identity:** scanner send fails loudly; no contact.
- **Owner has no DPNS name:** `build_auto_accept_qr` errors at QR-create (v1 requires `du`).
- **Queue flood:** bounded per owner; junk cleared by local verify before any fetch.
- **Expiry index overflow (> 2^31−1):** rejected at encode (and verify path-derivation errors → permanent-clear).

## 8. Test plan
- **Rust unit (auto_accept.rs):** key-blob round-trip; URI round-trip; **cross-actor** sign
  (loose key, scanner) → verify-with-pubkey (owner's re-derived pubkey) succeeds; wrong
  sender/owner/accountRef fails; expiry extraction + now ≤/≥ expiry; truncated/oversize/bad
  key-type rejected; structural pre-check.
- **Rust flow (contact_requests.rs):** parser reads `autoAcceptProof`; ingest-with-proof →
  `AutoAccept` enqueued (and bounded — Nth+1 dropped); drain valid+unexpired → reciprocal
  sent + cleared; expired → cleared, not accepted; invalid → cleared; signerless/transient →
  stays queued; **signed `accountReference` == document's** (scope M3).
- **FFI:** null/oversize/bad-URI input validation; build-QR → parse round-trip; drain with
  the new signer handle.
- **Swift build:** `build_ios.sh` green.
- **On-device (two sims):** A "Show my QR" → B scans → sends; A unlock/drain → contact
  auto-accepts (established, no tap on A); expired-QR path rejected.

## 9. Decisions (resolved 2026-06-24)
1. **TTL = 1 hour, fixed** (named constant `AUTO_ACCEPT_TTL_SECS = 3600`). DIP-15 is silent
   on the value (only mandates the timestamp *is* the expiry); 1h is the safe default given
   auto-accept is always-on (no off-switch). No picker in v1.
2. **Auto-accept = always automatic.** No opt-in toggle. Valid + unexpired proofs
   auto-accept in the drain.
3. **Scope = whole feature in one pass** (Rust + FFI + Swift + on-device), committed in
   logical layers on the branch.
4. **`du`-only** (no `di=` fallback); require a DPNS name to build a QR.

## 10. Review resolutions (4-lens, 2026-06-24)
- **DIP-fidelity:** wire-faithful, no byte fixes; pinned BE + SHA256/LE as normative (§3).
- **Security:** 4 must-fixes folded (§6); private-key-in-QR accepted as DIP-intrinsic,
  mitigated by the 1h TTL.
- **Feasibility (§4 rewrite):** verify via `provider.receiving_xpub(path).public_key` (no
  `&Wallet`); drain gains the identity signer + FFI `signer_handle`; the sweep parser must
  read `autoAcceptProof`; use `resolve_name`. Queue variant change-list = 9 sites.
- **Scope:** `du`-only + fixed TTL (cut `di=`/picker); cross-actor + `accountReference`
  ordering tests; banner counts `AutoAccept`; My-QR + Scan are net-new UI; clean stale
  `auto_accept.rs` docstrings.
