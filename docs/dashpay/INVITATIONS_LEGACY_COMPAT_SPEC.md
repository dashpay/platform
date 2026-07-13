# DIP-13 Invitations — Legacy Wallet Compatibility Spec (v2, review-folded)

Status: **REVIEWED (4 spec agents) → for sync/implementation** · Direction: **full legacy interop, dedicated PR** (do not fold into #4041; #4041 lands as the green baseline). · Author: platform-wallet

> v2 folds the four spec reviews (feasibility / scope / security / interop-correctness). See §11 for the resolution log. The headline correction: **the interop contract is "emit strict/canonical, parse leniently — exactly as tolerantly as the live Android wallet."** "Byte-for-byte parity" was wrong; it's **field-level** parity.

## 1. Problem & goal
PR #4041 shipped a new, self-contained invitation impl (`dashpay://invite?data=<binary envelope>`, custom scheme, reclaim) on the unified Rust/Platform-SDK stack. It is **wire-incompatible** with the two legacy wallets (`dash-wallet` Android / AppsFlyer, `dashwallet-ios` / dead Firebase), which share a **field-name-based** `du`+`assetlocktx`+`pk`(WIF)+`islock` payload. Goal: make new-stack invites **cross-claimable with the live Android wallet** by adopting the legacy payload + transport + onboarding amounts, while **keeping reclaim + seedless + one shared codebase**.

The on-chain primitive and derivation path (`m/9'/coin'/5'/3'/idx'`) are already identical across all three — no consensus change.

## 2. Goals / Non-goals
**Goals:** G1 payload cross-claim parity (field-level); G2 AppsFlyer OneLink transport; G3 onboarding amounts; G4 preserve reclaim + seedless.
**Non-goals:** removing the bearer model (self-custody); changing the on-chain primitive/path. (Our-own-AASA Universal Links, prior issue #4096, is superseded by the AppsFlyer decision.)

## 3. Wire format — the interop contract (FIELD-LEVEL parity; emit strict, parse lenient)

**Emit (canonical, what we produce):**
```text
dashpay://invite
  ?du=<inviter DPNS username>                 # required
  &assetlocktx=<funding txid, lowercase BIG-ENDIAN display hex>
  &pk=<voucher credit-burn key, WIF, COMPRESSED, network-correct>
  &islock=<InstantSend lock, lowercase hex>   # or omit (see below)
  [&display-name=<inviter display name>]
  [&avatar-url=<inviter avatar url, single %-encoded>]
```
- Parse **by field name, order-independent** — the two legacy wallets differ in param order *and* in scheme/host (iOS emits `https://invitations.dashpay.io/applink?…`), so **byte equality is not the contract**. Our parser MUST accept **both** the `dashpay://invite` scheme and the `https://invitations.dashpay.io/applink` host.
- **`pk`**: WIF, **compressed** flag set (the credit-output hash uses the *compressed* pubkey — wrong compression ⇒ wrong `hash160` ⇒ claim fails), network byte `0xCC` mainnet / `0xEF` testnet (matches bitcoinj + `dashcore::PrivateKey::from_wif`).
- **`assetlocktx`**: we **emit** lowercase big-endian display hex. On **claim** we parse leniently: try as-given, then **retry byte-reversed** on a fetch miss (mirrors Android's `Sha256Hash.wrap(id).reversedBytes` retry — old iOS links are little-endian and still exist).
- **`islock`**: OPTIONAL. Two absence forms MUST be handled: param missing, **and the literal string `"null"`** (Android emits `"null"` when the lock was a **chainlock**, not instantsend; its own `isValid` passes it). `islock == "null"` ⇒ treat as "no instant lock" and reconstruct a **`ChainAssetLockProof`** at claim (see §4.3), not an instant proof. iOS ignores `islock` on claim entirely.
- **islock version**: the hex is not self-describing — the decoder MUST assume **ISDLOCK** version and fall back to **ISLOCK** (Android does exactly this).
- **Validity (lenient, superset of both wallets):** require `assetlocktx` + `pk` present/non-blank (iOS's minimum). `du` is required to *emit* but treated as optional on *parse* (iOS accepts `du`-less links). Never reject solely on a missing/`"null"` `islock`.

## 4. The four changes

### 4.1 Payload codec — Rust `rs-platform-wallet/src/wallet/identity/crypto/invitation.rs`
Replace the binary envelope with the query form.
- `encode_invitation_uri` → build the §3 link. WIF via `dashcore::PrivateKey::to_wif` with `compressed=true`; assert compression+network in tests. txid via `proof.transaction().txid().to_string()` (lowercase big-endian). islock via consensus-encode of `proof.instant_lock()` → hex (omit if the proof is a ChainLock).
- `parse_invitation_uri` → accept both scheme + https host; parse the six fields by name; WIF network-checked decode; handle `islock` missing/`"null"`; **do not** fail on order or on missing `islock`/`du`.
- `ParsedInvitation` drops the embedded `asset_lock`; gains `funding_txid`, `islock: Option<…>`, and keeps `voucher_key` (decoded from WIF). Amount is **no longer known pre-fetch** — the preview shows amount only after §4.3 fetch (or "—" offline).

### 4.2 Transport — AppsFlyer OneLink (app layer)
- Wrap the inner `dashpay://invite?…` as `af_dp` in a OneLink; inbound conversion listener extracts `af_dp`/`deep_link_value`/`link` → claim flow. Keep the raw custom scheme as a **first-class parallel fallback** (QR / in-person), not an afterthought.
- **External blocker (does not gate G1/G3/G4):** OneLink brand domain (`dashpay.onelink.me`), dev key, template ID — from the Android team's AppsFlyer account; the iOS app must be added to the **same OneLink template**. Build with config placeholders; the custom-scheme path is fully testable without creds.
- **Threat-model note (§7):** AppsFlyer receives the plaintext `pk` server-side.

### 4.3 Claim — fetch tx by txid — Rust claim path
- Add `Sdk::get_transaction(txid) -> Transaction` (~20–40 lines; the gRPC `getTransaction` is already issued in `rs-sdk/src/core/transaction.rs:176` but the tx bytes are discarded — surface them, `Transaction::consensus_decode`).
- Reconstruct the proof mirroring Android `TopUpRepository.obtainAssetLockTransaction`:
  1. Fetch tx by `assetlocktx` (with the reversed-retry from §3).
  2. **Fail-fast guards:** `hash256(fetched tx) == assetlocktx` (both orders); if an islock is present, `islock.txid == fetched tx.txid`.
  3. **Derive `output_index` (do NOT hard-code 0):** reuse the shipped `voucher_credit_script(pk)` = `P2PKH(hash160(compressed pubkey(pk)))` as the **selector** — scan the fetched tx's `credit_outputs` for the output whose `script_pubkey` matches; reject if none matches. (The link carries `pk` but not the index; hard-coding 0 fails any legacy invite whose credit output isn't at index 0.)
  4. Build the proof: `islock` present → `InstantAssetLockProof { instant_lock, transaction, output_index }`; `islock == "null"`/absent → **`ChainAssetLockProof { core_chain_locked_height, out_point, transaction }`** (fetch the confirming height; mirrors the chainlock-only legacy path).
  5. Pass into the **unchanged** `put_to_platform_and_wait_for_response_with_private_key`.
- Retry/backoff on DAPI propagation lag (islock/chainlock proves finality). Consensus enforces pk↔output, islock↔tx, identity_id↔outpoint — all fail closed; the local guards are for fast-fail + correct index, not theft prevention.

### 4.4 Amounts — onboarding tiers — Rust constants + Swift UI
- **Normal tier (ship in v1): `0.03 DASH`** = identity + a normal DPNS name (Android `DASH_PAY_FEE`). New create **default = 0.03**.
- **Raise/drop `MAX_INVITATION_DUFFS`** — current `0.01` is **below** the normal tier, so create rejects its own default; set MAX ≥ the contested tier or drop the hard cap and gate on wallet balance (Android: `spendableBalance >= amount`).
- **Keep `MIN_INVITATION_DUFFS = 0.003`** (already == Android `DASH_PAY_INVITE_MIN`).
- **Contested tier `0.25 DASH` — DEFERRED** until contested/premium-name registration is actually wired into the new-stack claim flow (scope review F5; don't ship a price tier with no consumer). Add the tier + picker when that lands.

## 5. Preserved (unchanged): reclaim (outpoint + `funding_index`, orthogonal to link format), seedless key handling, shared Rust core. Regression-test reclaim after the codec change.

## 6. Decisions — resolved
- **D1 contact bootstrap:** KEEP opt-in-both-ends (prior owner decision, safer for a bearer link); accept the minor parity gap vs legacy's auto-one-way. (Revisit if product wants frictionless parity.)
- **D2 AppsFlyer creds:** external blocker for G2 only; G1/G3/G4 proceed.
- **D3 byte order:** emit big-endian; **parse lenient with reversed-retry** (do NOT drop the hack — that was a regression).
- **D4 WIF:** compressed=true, network byte `0xCC`/`0xEF`; cross-wallet WIF byte-equality fixture test.
- **D5 sequencing:** **dedicated PR** stacked on the invitation branch; #4041 lands as baseline.

## 7. Threat model & failure modes
| Risk | Severity | Mitigation |
|---|---|---|
| **AppsFlyer discloses plaintext `pk` server-side** | MED (regression vs #4041) | Bounded by amount cap + fast reclaim (NOT the advisory expiry, which a direct holder ignores). Document AppsFlyer as an untrusted intermediary; ensure the web preview forwards only `display-name`/`avatar-url`, never the `pk`-bearing link. |
| Claim-time tx fetch fails (DAPI lag) | MED | Retry/backoff; islock/chainlock proves finality; clear "still confirming" state. |
| Wrong endianness / can't claim old iOS links | LOW | Reversed-retry on fetch miss (§4.3). |
| WIF compression/network mismatch → silent claim fail | LOW | Compressed+network round-trip test. |
| `islock="null"` chainlock invite unclaimable | MED | ChainAssetLockProof path (§4.3.4). |
| Attacker-crafted link | — | No theft (consensus fails closed); worst case a failed claim (griefing) or a valid self-controlled identity. |
| Reclaim vs new payload | — | Orthogonal; regression-tested. |

## 8. Architecture / ownership
Rust `rs-platform-wallet`: codec (§4.1), claim-by-fetch (§4.3), amount constants (§4.4). `rs-sdk`: `get_transaction` wrapper. FFI: signature updates. Swift/SwiftExampleApp: create tier UI, AppsFlyer wrap/inbound (§4.2), claim wiring. Reclaim: untouched.

## 9. Test plan
- **Rust unit:** WIF **compressed+network** round-trip; `assetlocktx` big-endian emit + reversed-retry parse; `islock` present / absent / `"null"` handling; field-name parse (order-independent, both scheme + https host); `output_index` selection by `pk` match; ISDLOCK→ISLOCK fallback.
- **Interop fixture:** parse a **real captured Android link** (incl. a `"null"`-islock case) and assert **field equality** (NOT byte equality) + a successful claim reconstruction; if obtainable, an old little-endian iOS link.
- **Reclaim regression:** create → reclaim (topup + register) still green post-codec.
- **Funded testnet e2e (two emulators):** new→new claim (fetch path); new→**live Android `dash-wallet`** cross-claim (the real proof, when a build is available); reclaim-vs-claim race unchanged; amount 0.03 funds identity + a normal name end-to-end.
- **CI:** `cargo fmt --all --check` + workspace tests + Swift SDK strict build.

## 10. Rollback: revert the codec/claim/amount commits; tracked invitations (outpoint-keyed SwiftData) unaffected.

## 11. Spec-review resolution log
- **Feasibility:** claim-by-fetch is a ~20–40 line wrapper (gRPC already called); WIF exists & network-aware; amounts trivial; AppsFlyer the only external blocker. → §4.3, §4.1, §4.4, D2/D4.
- **Interop-correctness (highest-impact):** "byte-for-byte" → **field-level**; **keep** endianness reversed-retry; handle **`islock="null"`**; **compressed** WIF; accept both scheme+https host; islock optional; ISDLOCK/ISLOCK. → §3, §4.1, §4.3, D3/D4.
- **Security:** no theft vector (consensus fails closed); **must** derive `output_index` by `pk` match (not index 0); add `txid`/`islock.txid` fail-fast guards; **`islock="null"` needs ChainAssetLockProof**; **AppsFlyer server-side `pk` disclosure** is a real regression → threat model. → §4.3, §7.
- **Scope:** amount fix is a standalone bug (ship regardless); **#4041 stays green baseline**; interop = **dedicated PR**; **defer the 0.25 contested tier**; AppsFlyer re-introduces the 3rd-party-service dependency class → keep custom-scheme fallback first-class. → §2, §4.2, §4.4, D5.
