# DashPay (DIP-15 / DIP-16) QA test-case expansion — SPEC

**Status:** reviewed (4-lens multi-agent pass folded in)
**Target file:** `packages/swift-sdk/SwiftExampleApp/TEST_PLAN.md` §4.10 (+ §5, §6, §1)
**Base:** branched off `feat/dashpay-m1-sync-correctness` (PR #3841,
"fix(platform-wallet)!: complete dashpay") — the DashPay views, `docs/dashpay/`,
and the features these rows describe live there, not yet in `v3.1-dev`.
**PR target:** `v3.1-dev`, to merge **after** #3841 lands (pure-docs diff;
rebased so it shows only the TEST_PLAN.md / spec changes).
**Renders in:** [`dashpay/qa-dashboard-site`](https://github.com/dashpay/qa-dashboard-site)
once the sibling seed task re-seeds the `dash-qa` contract from the updated plan.

---

## 1. Problem

`TEST_PLAN.md` §4.10 (DashPay) had **6 coarse rows** (`DP-01..06` + cross-ref
`MW-03`) at "feature exists" granularity, and their entry points were **stale**:
they cited `FriendsView` / `AddFriendView`, which #3841 replaced with a dedicated
DashPay tab (`DashPayTabView`, `AddContactView`, `ContactsView`,
`ContactRequestsView`, `ContactDetailView`, `DashPayProfileView`,
`IgnoredContactsView`, `SendDashPayPaymentSheet`).

#3841 implements substantial **DIP-15** surface the catalog did not exercise
(per the branch's audit `docs/dashpay/DIP_CONFORMANCE_GAPS.md`):
`encryptedAccountLabel` send+receive, QR auto-accept (build + paste-to-add),
on-chain `contactInfo` publish, and the §12.6 incoming-payment backfill rescan.
**DIP-16** is the SPV sync layer underneath (covered by `CORE-07`; its
DashPay-specific facet is the single backfill row `DP-10`).

## 2. Goal & non-goals

**Goal:** correct the stale `DP-01..06` entry points and add rows for the
**user-observable, simulator-drivable** DIP-15/16 DashPay flows #3841 ships.

**Non-goals (out of scope):**
- **DIP-15 crypto internals** (ECDH, 69-byte compact xpub, `accountReference`
  masking, avatar hash/dHash) — already Rust known-answer tests; not app rows.
- **Gap / absence rows** (`🚫`/`➖`) for unimplemented features (multi-account
  `Account≠0`, `acceptedAccounts` flood mitigation, invitations). Drivable only.
- **A DIP-16 section.** SPV sync is `CORE-07`; the DashPay facet is `DP-10`.
- **New table columns.** Keep the uniform 6-col `ID | Action | Layer | Tier |
  Status | Entry point & test notes`; cite DIP §s inline. (The dashboard
  normaliser reads no section field; a 7th column is dropped on seed.)
- **A `tags` column.** Tag assignment for the v5 contract is the seed tool's job
  (not in these repos). Open question §7.

## 3. Corrections to existing rows (`DP-01..06`)

Entry points updated to the #3841 DashPay tab; FFI-symbol naming (matches the
existing §4.10 convention). Merged sub-flows folded in as notes:
- **DPNS-add path** → a note on `DP-01` (precedent: `ID-04`/`MW-01` list input
  methods in one row; DPNS resolution itself is `DPNS-03`/`DPNS-07`).
- **Payment-channel-broken** state → a note on `DP-03` (precedent: `ID-12`/`DOC-07`
  attach gating state to the action row).
- **Avatar** (url + Rust-computed hash/fingerprint) → a note on `DP-04`.
- `DP-06` **Reject → Ignore** rename (the branch made reject a reversible local mute).

## 4. New rows (drivable DIP-15/16 flows)

| ID | Tier | DIP-15 § | Behavior |
|---|---|---|---|
| DP-07 | Common | §8.5 | Attach `encryptedAccountLabel` on send; counterparty sees "Their account" (decrypted, incoming-row only). |
| DP-08 | Thorough | §8.13 | QR auto-accept: build "Add me" QR; add via pasted URI → auto-accepted without manual accept. Paste-drivable; camera = Manual variant. |
| DP-09 | Thorough | §10 | Publish encrypted on-chain `contactInfo`; ≥2-contact gate → `.published` / `.deferredUntilTwoContacts` / `.skippedWatchOnly`. |
| DP-10 | Manual | §8.7/§12.6 | Incoming-payment backfill rescan (no UI trigger; `reconcile_dashpay_rescan` rewinds SPV `synced_height`). Env-limited; the §12.6 payment-loss regression pin. |

`DP-10` note: the branch's `DIP_CONFORMANCE_GAPS.md` §1.1 still marks this MISSING,
but that audit predates the implementing commit `18483e4232`
(`reconcile_dashpay_rescan`, wired in `manager/dashpay_sync.rs`, 4 unit tests) —
so Status=✅ is correct.

## 5. Cross-cutting edits (applied)

- **§6 index** — DashPay: `DP-01..06, MW-03` → `DP-01..10, MW-03`.
- **§5 by-tier** — Common `31→32`, Thorough `35→37`, Manual `1→2` (`CORE-08, DP-10`).
- **§5 by-layer (automatable; Manual EXCLUDED)** — Platform `~72→~75`; **Cross
  unchanged** (DP-10 is Manual).
- **§1 worked example** — "list the manual tests" → `CORE-08, DP-10`.

## 6. Final row set

6 corrections (`DP-01..06`) **+ 4 new** (`DP-07` account label, `DP-08` QR
auto-accept, `DP-09` on-chain `contactInfo`, `DP-10` backfill rescan).

## 7. Open questions

1. **Tags** — does the seed tool assign v5 tags (e.g. `dip15`, `sync`) from
   Domain/§6, or should the plan encode them? Needs the seed tool (not in repos).
2. **DP-07 a11y id** — the "Their account" block in `ContactDetailView` has no
   `accessibilityIdentifier`, so DP-07 asserts on visible text. A 1-line app
   change would make it cleanly automatable — a trivial follow-up, deliberately
   kept out of this docs-only PR.

## 8. Verification plan

1. **Render check** — IDs match `^DP-\d+$`; tier/category present so the dashboard
   matrix charts them (the normaliser only hard-requires `testId`).
2. **Drive each new row** with the `simulator-control` skill on a booted sim; two
   on-device wallets where a counterparty is needed (`DP-07`/`DP-08`, cf. `MW-03`).
   Verify against **persisted SwiftData state**, not UI alone (§1 pass criteria).
   `DP-10` is Manual → skip-and-flag in automation.
3. **No code change** — pure TEST_PLAN.md edit. The seed task re-seeds `dash-qa`;
   the dashboard renders.

## 9. Review provenance

Four independent review lenses (DIP domain-fit, scope/simplicity, automatability/
entry-point accuracy, catalog conventions) ran against the draft. Key folds:
- Added `DP-09` (on-chain `contactInfo`) — the draft wrongly excluded it as
  "local-only / no UI"; it is a drivable DIP-15 §10 publish.
- Merged the draft's separate DPNS / avatar / channel-broken rows into notes on
  `DP-01` / `DP-04` / `DP-03` (catalog precedent; lean set).
- Dropped a 7th `DIP-15 §` column (normaliser ignores it; breaks the 6-col shape).
- Confirmed `DP-10`'s rescan is implemented + wired; corrected the `wallet.pass`
  SF-symbol-vs-a11y-id confusion in `DP-07`.

## 10. Runtime verification (simulator)

Driven on a booted iOS simulator (iPhone 17) against a live devnet build with real
DashPay fixtures (wallet "SimB", 1 identity, 2 contacts, 5 requests), via the
`simulator-control` skill. Read-only structural pass — navigated to each row's
entry point and confirmed the cited screens/controls exist; **no broadcasts fired**.

Confirmed live:
- `DP-01` — `AddContactView` mode picker + resolved-recipient preview + **Send Request**.
- `DP-03` — `ContactDetailView` `dashpay.detail.sendDash`.
- `DP-04` — `DashPayProfileView` **Edit** (→ editor) + avatar.
- `DP-05` — DashPay tab: `ContactsView` (search, contacts, segment, profile header).
- `DP-07` (send) — `dashpay.addContact.accountLabel` renders once a recipient resolves.
- `DP-08` — build: `dashpay.profile.qrURI` emits a real `dash:?du=…&dapk=…` URI + QR
  image; add: `AddViaQRSheet` `dashpay.qr.uriField`.
- `DP-09` — the **Alias / Note / Hide** editor calls `saveContactInfo` →
  `setDashPayContactInfo`; the in-app footer confirms the ≥2-contact encrypted-publish
  gate. **Refined the row** accordingly — the original "distinct from a local note"
  wording was wrong (the same editor caches locally *and* publishes on-chain).

Code-confirmed but not rendered this pass (no fixture): `DP-02` / `DP-06`
(`dashpay.request.accept` / `.ignore` — need an *incoming* pending request);
`DP-07` receive-side "Their account" (only shows when a contact sent a label);
`DP-10` (no UI by design — automatic in DashPay sync). Live broadcast execution and
the two-wallet loops (`DP-07`/`DP-08`) are the next step, gated on credits + a
counterparty identity.

A full code re-audit of every row (4 parallel passes) confirmed 8/10 rows + all the
§5/§6/§1 count edits accurate, and corrected 5 row-wording inaccuracies: DP-01 (open
button id `dashpay.addContact` vs the in-sheet mode toggle), DP-02/DP-05
(`EstablishedContact` is a Rust/FFI handle, **not** a SwiftData model — the tab views
are backed by `PersistentDashpayContactRequest`), DP-03 (channel-broken is any
permanent channel failure, not only key rotation), and DP-08 (TTL is exactly 3600s).

## 11. Implementation observations (for #3841 — surfaced during verification, NOT addressed here)

These are defects/smells in the DashPay *implementation* found while auditing the
plan. They are out of scope for this docs PR; recorded for the #3841 author.

1. **Stale doc-comment** — `ContactRequestsView.swift:5-8` says incoming rows carry
   "**Accept / Reject**", but the button is **Ignore** (reject was replaced by the
   reversible local mute). Same file `:35-37` carries an internal `§6.4` spec-gate
   ref (rots; against the timeless-comment convention).
2. **Multi-wallet mis-attribution risk** — `DashPayProfileEditorView` falls back to
   `walletManager.firstWallet` when `walletId` is nil (`IdentityDetailView.swift:1316`);
   in a multi-wallet setup a profile update could submit under the wrong identity.
   Already acknowledged in an in-code comment as needing tightening.
3. **Handle-leak smell** — `acceptContactRequest`'s returned `EstablishedContact`
   (FFI handle wrapper) is discarded with `_ =` at both call sites
   (`ContactRequestsView.swift:228`, `AddContactView.swift:487`); leaks per accept
   unless the wrapper frees the handle in `deinit` (worth confirming a `deinit`).
4. **QR clock edge** — `build_auto_accept_qr` derives expiry from
   `SystemTime::now()…unwrap_or(0)`; a pre-1970 / badly-skewed clock yields an
   already-expired QR. Harmless on a real device.
5. **No collision handling in `AddViaQRSheet`** — pasting a URI from someone who
   already sent *you* a request broadcasts a duplicate outgoing request rather than
   offering "Accept instead" (`AddContactView` handles this; the QR path does not).

By-design / cosmetic (no action expected): avatar hash does not change if the image
bytes are swapped behind the same URL; a corrupt/hostile incoming account label
unpads to garbage and is coerced to `None` (shows no "Their account" — relevant to
DP-07 negative testing); `setDashPayContactInfo` maps unknown future outcome bytes to
`.published` on the Swift side; a stale memo doc-comment in `SendDashPayPaymentSheet`
(DashPay payments always pass `memo: nil`); a dead `_ = bytes` local + a redundant
`?? nil` duplicated across four profile-cache reads.

## 12. Live end-to-end run (freshly-built binary)

Built `build_ios.sh --target sim` from `feat/dashpay-m1-sync-correctness` HEAD
(`47d9044b5a`), installed on two iOS simulators, and drove the flows on-chain
against devnet (two funded identities per side): **Eve** (SimB) ↔ **Alice / Bob /
Dolly(7A8E)** (SimA), each ~25–30B credits. Verified against SwiftData ground
truth (and on-chain for the payment).

| Row | Result (fresh build) | Evidence |
|---|---|---|
| DP-01 send | ✅ | labeled contact request broadcast (sheet dismissed, no error); also the DP-02 reciprocal send |
| DP-02 accept | ✅ | 7A8E accepted Eve → reciprocal `7A8E→Eve` row created (established) |
| DP-03 payment | ✅ one direction | Eve→Alice **0.001 DASH** real L1 tx (input spent, change `74,899,477`, fee `226` duffs, txid `850433507c88…560e`) — **after starting Core SPV**. ⚠️ only the forward direction was driven; see the bidirectional gap below |
| DP-04 profile | ✅ | publicMessage updated on-chain → SwiftData (`QA fresh-build 16:10`) |
| DP-05 view | ✅ | contacts / requests / profile rendered throughout |
| DP-06 ignore | ✅ | registered a fresh identity (asset-lock funded, ChainLock proof) → sent Eve a request → Eve **ignored** it (→ ignored-senders) → **un-ignored** (reversed). Local-only mute |
| DP-07 label | ✅ fresh first-contact | Bob→**EveN** (fresh pair) labeled send; EveN accepted → decrypted "Their account" = the sent label. Confirms decrypt-on-accept end-to-end |
| DP-08 QR | ✅ fresh first-contact | Alice built `dash:?du=…&dapk=…`; **EveN** pasted + `sendContactRequestFromQR`; Alice **auto-accepted** (reciprocal, no manual Accept) — *after unlocking Alice's wallet* (signer-backed drain; see note) |
| DP-09 contactInfo | ✅ on-chain | log: `Published contactInfo document identity=Eve contact=Alice` — the `.published` outcome, not just local persist |
| DP-10 backfill rescan | ✅ mechanism (logs) | the §12.6 rescan fired live: `DashPay rescan: lowered SPV synced_height … floor=51112` → `dash_spv…filters: synced_height 51112 fell below committed_height 52175, restarting scan`. No UI trigger (Manual tier); the full restore-from-seed payment-recovery remains a device exercise |

**10/10 flows verified live on the fresh build** — DP-01..09 driven on-chain
(SwiftData + chain; DP-07/DP-08 via a freshly-registered unconnected identity to
get clean first-contact pairs), DP-09's on-chain publish + DP-10's backfill-rescan
both confirmed in the Rust logs.

**Gap (DP-03 bidirectional):** only the **forward** payment (Eve→Alice) was driven.
The reverse (Alice→Eve) is symmetric by design — once established, each party derives
the other's payment address from the exchanged xpubs — but it was **not** verified
live (SimA's app context had flipped to a separate testnet wallet set). DP-03 now
explicitly requires verifying **both** directions; the reverse remains to be driven.

**Finding (DP-08):** the QR auto-accept *reciprocal* is signer-backed, so it only
fires once the recipient's wallet is **unlocked** (the "N contacts waiting to finish
setup → Unlock" drain). The request and auto-accept proof reach the recipient
immediately, but the established reciprocal lands after unlock — so "auto-accept" is
not fully hands-off. Worth surfacing in DIP-15 §8.13 expectations.

Two plan corrections came out of the run: **DP-03** now records the Core-SPV
precondition (a DashPay payment is an L1 broadcast — fails "SPV Client not started"
if SPV is stopped); **DP-07** now states the label decrypts **on accept**, not on
ingest.
