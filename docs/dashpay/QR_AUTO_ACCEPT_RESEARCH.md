# DashPay QR Auto-Accept — Research Findings (DIP-15 + reference-client audit)

Date: 2026-06-24. Scope: should we wire the dormant `autoAcceptProof` /
`auto_accept.rs` into a QR-based contact flow? Sources: canonical DIP-0015 +
DIP-0013, `dashpay/dashj`, `dashpay/android-dashpay`, `dashpay/dash-wallet`
(GitHub code search + raw reads), and our in-repo code.

## Verdict (read this first)

**QR auto-accept via DIP-15 `autoAcceptProof` is unimplemented in every reference
client — it is spec-only, dormant everywhere, with no interoperable counterparty.**
Building it would mean inventing an iOS-only convention that no other Dash client
verifies. **Recommendation: do not build QR auto-accept now.** If the underlying
goal is "scan/tap to add or onboard a friend," the real shipped, interoperable
feature is **Invitations** (a *separate* mechanism — see §5), not `autoAcceptProof`.

## 1. Reference-client status — dormant everywhere

| Client | `autoAcceptProof` status |
|---|---|
| `dashpay/dashj` (L1/SPV + HD wallet) | **Zero references.** No `ContactRequest` model at all — DashPay docs aren't in dashj. The payment-channel path `9'/5'/15'` *is* implemented (`FriendKeyChain`); the auto-accept path `16'` is not. |
| `dashpay/android-dashpay` (the Kotlin DashPay SDK) | Field **defined** (`ContactRequest.kt`) + an unused builder `autoAcceptProof(...)` with **0 callers**. `ContactRequests.create()` never sets it. No signing, no verification, no auto-accept-on-receive (no `accept` method exists; `watchContactRequest` only polls + callbacks). |
| `dashpay/dash-wallet` (Android app) | Field is a **dormant Room DB pass-through** (`DashPayContactRequest.kt` + migrations 13–18): read from the inbound doc, forwarded to the builder, **never constructed, never used to skip approval**. The QR scanner (`ScanActivity` → `InputParser`) only handles payments / WIF sweep / raw tx — it **cannot add a contact**. Adding a contact is username-search → manual send → manual accept (acceptance = sending a reciprocal request). |

**Interop implication:** sending `autoAcceptProof` gains nothing (no client verifies
it); omitting it costs nothing (it's optional, the reference SDK never sets it). An
iOS implementation would only ever auto-accept against *another copy of our own SDK*.

## 2. What DIP-15 actually mandates vs leaves open

**Mandated (wire-fixable):**
- `autoAcceptProof` is an optional `contactRequest` byteArray, **38–102 bytes**
  (matches our `packages/dashpay-contract/schema/v1/dashpay.schema.json`, not in `required`).
- **QR key blob** (the `dapk` URI param): `key type (1) | timestamp (4) | key size (1) | key (32–64)`.
- **On-document proof blob**: `key type (1) | key index (4) | signature size (1) | signature (32–96)`.
- **Derivation path**: `m / 9' / 5' / 16' / timestamp'` — feature fixed at `16'`,
  the final hardened index **is the expiry timestamp** (≤ 2^31−1 → bounded ~2038).
- **Signed pre-image content**: `$ownerId + toUserId + accountReference`.
- **URI scheme** (BIP21/BIP72 extension): `du` = username, `dapk` = the auto-accept
  key blob. E.g. `dash:?du=bobspizza&dapk=…` (contact) or with `amount=…` (merchant).

**Left implementation-defined (the spec is silent — security-critical half):**
- The **recipient-side verification algorithm** (the whole thing).
- The **signature scheme** + the `key type` byte's meaning (ECDSA vs BLS not enumerated).
- The **exact serialization** of the signed pre-image (raw concat vs hashed; field
  encodings; endianness of `accountReference`). ← byte-level interop is undefined.
- Whether **expiry is enforced** (only "essential to verify" is stated, not a reject rule).
- Any **nonce / one-time-use / replay** protection (none; the only structural defense
  is that `($ownerId, toUserId, accountReference)` is the doc's immutable primary key).

## 3. The trust model (corrected)

Earlier reasoning here assumed the *counterparty* signs the proof, making our
self-deriving `verify` look broken. The DIP-15 mechanism is the opposite and our
self-verify is actually **correct**:

- The **QR shower** (merchant/host, "Bob") derives an auto-accept key at
  `m(Bob)/9'/5'/16'/timestamp'` and **puts the 32-byte *private* key in the QR**
  (`dapk`; size field 32 ⇒ a private key, handed out deliberately, expiry-bounded).
- The **scanner** signs `$ownerId ‖ toUserId ‖ accountReference` with that handed-out
  key and attaches the proof to the contact request they send to Bob.
- **Bob (recipient) re-derives the same key from his own seed** (he knows `timestamp`
  from the proof), gets the pubkey, checks the signature → **self-verification against
  his own key is the intended model.** So `verify_auto_accept_proof(wallet, …)` is right.

Security note: because the QR hands out a usable private key, *anyone* who sees the
QR before it expires can get auto-accepted as Bob's contact. That's acceptable for
the merchant/proximity use case (low stakes: it only establishes a contact channel),
but it's a deliberate trade-off, not an oversight.

## 4. Our current Rust code vs DIP-15

`packages/rs-platform-wallet/src/wallet/identity/crypto/auto_accept.rs` (tested, dormant):
- ✅ Proof **structure** matches (`key_type | index | sig_size | sig`).
- ✅ Derivation path `m/9'/coin'/16'/timestamp'` matches.
- ✅ `verify` (self-derive from the recipient's wallet) matches the corrected model.
- ⚠️ **Signed bytes**: we use `SHA256($ownerId ‖ $toUserId ‖ accountReference_LE)`.
  DIP names the same three fields but does **not** specify hashing/encoding, so this
  is *our* convention — unverifiable against a reference (none exists).
- ❌ **Scanner-side signing is the wrong actor**: `generate_auto_accept_proof` derives
  the key from a full wallet seed (models *Bob*), but per spec the *scanner* signs with
  the loose QR-provided key. There is no "sign-with-provided-32-byte-key → proof" fn.
- ❌ **QR encode/decode** (`dapk`/`du` URI, the key blob layout) doesn't exist.
- ❌ **Expiry enforcement** (reject when `now > timestamp`) isn't done.
- ❌ **No FFI** exposes generate/verify; the accept path loads `auto_accept_proof` from
  chain but never verifies or acts on it.

So even the "we already have the crypto" framing is partial: the crypto we have models
the wrong actor for the scanner side, and the QR/URI + expiry + FFI + accept-hook are
all absent.

## 5. Invitations ≠ auto-accept (the real shipped feature)

The shipped "scan/link to bring a friend in" feature is **Invitations**, a *separate*
subsystem with no shared code:
- Protocol primitive: **DIP-0013 sub-feature `3'`** ("Identity Invitation Funding keys").
- dash-wallet impl: a **deep link** `dashpay://invite?du=…&assetlocktx=…&pk=<WIF>&islock=…`
  (web fallback `https://invitations.dashpay.io/applink?…`), handled by
  `InviteHandlerActivity` — **not** the QR scanner. Claiming rebuilds the
  `AssetLockTransaction`, decodes the embedded WIF, and calls
  `initializeAssetLockTransaction(...)` to **register a brand-new identity** for someone
  who has no Dash and no identity.
- Purpose: **onboarding** (fund + bootstrap an identity), not contact auto-acceptance.

If the product goal is "let an existing user invite a friend who isn't on DashPay yet,"
Invitations is the interoperable target — a much larger feature (L1 asset-lock funding +
identity registration from an embedded WIF), separate from this `auto_accept.rs` work.

## 6. Recommendation

1. **Do not wire QR auto-accept now.** It has no interoperable counterparty (every
   reference client leaves it dormant), the spec leaves the security-critical half
   undefined, and our crypto models the wrong actor for the scanner side. Keep
   `auto_accept.rs` as-is (tested, dormant); update the TODO to reflect "spec-only,
   no interop — deprioritized," not "ready to wire."
2. **If contact-onboarding is wanted**, scope **Invitations** (DIP-13 `3'`,
   `dashpay://invite` + AssetLock) as its own feature — that's what ships and interops.
3. Either way, the keep-it-honest fix already noted in `SPEC.md` Part 8.5 stands: *if*
   anything ever acts on `autoAcceptProof`, it MUST call `verify_auto_accept_proof`
   first. Nothing acts on it today, so there's nothing to gate yet.
