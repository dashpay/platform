# Shielded-pool TestFlight feedback — investigation

Tracking two TestFlight feedback reports against build **3** (`cfBundleShortVersion 1.0`, `cfBundleVersion 3`, appAppleId `6782996681`). Both concern the shielded (Orchard) pool. Source exports: `~/Downloads/testflight_feedback*.zip`.

## Reports

### Report B — shielded→Core withdrawal never confirms ("may have gone through") — PRIMARY
- **Feedback id:** `AHlzEsyX33yEvRlSE4Qtm34`, 2026-06-29T18:44 UTC
- **Device:** iPhone18,2 (iPhone 16-class), iOS 26.6, locale pl-PL, tz Europe/Rome, uptime ~398 s
- **Comment (verbatim):** "Yesterday I sent tx from shielded to core and had the same msg as above. Today I checked and the balance was not sent. Today I have again info it might go through but will see later if it gone through or not."
- **Screenshot:** "Send Dash" sheet, recipient a **Core address** (`yTDFHbti48ArHwTszgM…`), **Transaction Type: Withdrawal to Core**, **Send From: Shielded** (0.2719731 DASH selected). A modal titled **"Success"** with body **"Transaction may have gone through — waiting for the next shielded sync to confirm. Do not retry."** + a **Done** button.
- **Reproduced across two days** with the same outcome (balance not actually moved).

### Report A — switching wallets during a shielded sync — THIN
- **Feedback id:** `ACKbeJnBYOVKfN6dXEPFXG8`, 2026-06-28T22:35 UTC (uploaded twice — `testflight_feedback.zip` and `(1).zip` are byte-identical)
- **Device:** iPhone13,2 (iPhone 12 Pro), iOS 26.5, locale en-US, carrier AT&T, tz America/Los_Angeles
- **Comment (verbatim, truncated by TestFlight):** "Switching between wallets after starting a shielded sync "
- **No screenshot, no crash log.** Insufficient to reproduce yet; needs the full comment or a crash report.

## Mechanism traced (Report B)

The dialog is **not** an ad-hoc string — it is the deliberate handling of `PlatformWalletError::ShieldedSpendUnconfirmed`.

1. UI: `SendViewModel.swift:734-744` catches `PlatformWalletError.shieldedSpendUnconfirmed` and surfaces it through the **success** path (title "Success"), explicitly to avoid inviting a retry that could double-spend.
2. Rust: `operations.rs::classify_spend_wait_failure` (≈1896) classifies a post-broadcast `wait_for_response` failure:
   - `carries_consensus_rejection(wait_err)` → `ShieldedBroadcastFailed` (Platform executed + rejected on merits).
   - **otherwise** (DAPI timeout / internal error / `StateTransitionBroadcastError` with empty consensus data → `cause: None`) → **`ShieldedSpendUnconfirmed`** — the note reservations are **kept**; the next sync reconciles.

So the withdrawal **is broadcast**, then the wait-for-result fails **ambiguously** (no consensus verdict), and the reservation is intentionally held. The user hitting this **repeatedly** means the withdrawal ST's wait-for-result keeps failing ambiguously and the tx is not landing.

### Broadcast → wait → classify (precise path)

`operations.rs::broadcast_shielded_spend` (≈1785):

1. `state_transition.broadcast(sdk, None)`:
   - `Ok` → fall through to the wait.
   - `broadcast_definitely_failed(e)` (consensus rejection, gRPC verdict code, `NoAvailableAddresses`) → `ShieldedBroadcastFailed` → outer match **releases** the reservation.
   - **otherwise** (`AlreadyExists`, `DeadlineExceeded`, `Cancelled`, `Unknown`, `Internal`, `Aborted`, `DataLoss`, …) → warn "may have been admitted" → **fall through to the wait anyway**.
2. `state_transition.wait_for_response::<StateTransitionProofResult>(sdk, None)`:
   - `Ok` → Confirmed.
   - `Err` → `classify_spend_wait_failure`: consensus rejection → `ShieldedBroadcastFailed`; **else → `ShieldedSpendUnconfirmed`** (keep reservation).

The ambiguous bucket is deliberately wide — both a broadcast that *probably* failed (ambiguous transport error) and a wait that timed out land in `ShieldedSpendUnconfirmed`, on the conservative "a lost ACK may have delivered, so don't re-spend" principle.

## Funds safety (confirmed — no permanent loss)

`pending_nullifiers` is **in-memory only** (`operations.rs:757-764`):
- If the tx **landed** → the next shielded sync marks the spent notes spent (clears the reservation).
- If the tx **never landed** → an **app restart drops the in-memory reservation and frees the notes**.

So the notes are **not** permanently stranded — consistent with the user seeing the balance "back" the next day (app relaunch between sessions). The cost is UX: within a session the funds appear reserved/unavailable, the user is told "do not retry," and there is no clear resolution short of relaunch.

### Confirmed gap: no sync-time release of a stale reservation

The reservation set `pending_nullifiers` (`store.rs:348`) is only ever cleared by:
- `mark_spent` — when a later **sync proves the note spent** (the tx landed); or
- `clear_pending` — called by the spend-flow finalizer on a **definite** failure; or
- **process restart** — `pending_nullifiers` is in-memory only and is never persisted, so a relaunch rebuilds the store without it.

There is **no reconcile path that releases a reservation whose tx demonstrably never landed**. For the `ShieldedSpendUnconfirmed` outcome the reservation is held indefinitely within the session, the activity row stays `Pending`, and the UI shows "do not retry" — so a genuinely non-landing withdrawal **strands the notes until the user force-quits the app**. This is the highest-leverage, most testable defect in the report, independent of *why* the withdrawal failed to land.

## Severity

- **Not** fund-loss. Funds return to spendable after a restart + sync.
- **Reliability bug:** the shielded→Core withdrawal repeatedly fails to confirm (ambiguous wait failure) — the user cannot complete a withdrawal.
- **UX bug:** modal title "Success" contradicts "may have gone through"; "Do not retry" with no confirmation path leaves the user stuck; funds appear unavailable until relaunch.

## Open questions / hypotheses (Report B reliability)

1. **Where does the ambiguity originate?** Is `wait_for_state_transition_result` timing out at DAPI, or is the ST being rejected without a surfaced consensus verdict? (`classify_spend_wait_failure` treats both as "unconfirmed.")
2. **Does the ST actually reach mempool / execute?** If it never lands across the next sync, is it never accepted, or accepted-but-slow?
3. **Withdrawal-specific?** Do shield / unshield / shielded-transfer confirm fine while only "withdrawal to Core" (transparent recipient) stalls? Points at the transparent-output path or its proof/fee handling.
4. **Network:** which network was the reporter on (mainnet/testnet)? DAPI wait reliability differs.

## Investigation / reproduction plan

Code trace (done):
- [x] Read the withdraw-to-Core operation (`operations.rs` ≈930-1070) end to end: reserve → build → record-pending → broadcast → wait → classify.
- [x] Map `broadcast_shielded_spend` + `broadcast_definitely_failed` + `classify_spend_wait_failure` — the ambiguous bucket.
- [x] Confirm the reservation lifecycle: a non-landing reservation is released **only** on tx-landing or restart — no sync-time release. **This is the key fixable gap.**

Reproduction (needs a shielded environment + funds):
- [ ] Stand up / reuse a shielded devnet (see `~/.claude/.../reference-devnet-deploy-and-shielded-image-build.md`) or confirm which network the reporter used.
- [ ] Fund a shielded note; attempt a shielded→Core withdrawal via the Rust SDK / a focused harness with `tracing` at `debug`.
- [ ] Capture the exact failure shape: does `broadcast()` return Ok then `wait_for_response` time out, or does `broadcast()` itself return an ambiguous error? Capture the `wait_err`/transport code.
- [ ] Determine whether the ST ever reaches mempool / a block (does it land late, or never?) — distinguishes "slow confirm" from "genuine non-landing."
- [ ] For Report A: obtain the untruncated comment / any crash log; attempt a wallet-switch-during-shielded-sync repro in the simulator.

Two fix tracks (independent):
- **Reservation-staleness (high leverage, unit-testable now):** add a bounded sync-time release for a `ShieldedSpendUnconfirmed` reservation whose nullifier is still absent on chain after the spend can no longer be valid (anchor/expiry window elapsed) — free the notes **without** a restart and flip the activity to a retryable state. Testable at the Rust layer without the network.
- **Root cause (needs repro):** why the withdrawal ST does not land — broadcast transport, wait timeout sizing for proof-heavy shielded STs, or an execution-time rejection that never surfaces a consensus verdict.

## Root cause (B) — code investigation findings (testnet)

**Wait has no client timeout.** `withdraw` calls `wait_for_response(sdk, None)`; with `wait_timeout = None` the SDK waits *without* a client-side duration cap (`rs-sdk/.../broadcast.rs:246`). So "the wait timed out too soon" is **not** the cause — `ShieldedSpendUnconfirmed` fires only when the DAPI result genuinely never resolves.

**Leading hypothesis: anchor mismatch (`validate_anchor_exists`).** The drive-abci shielded-withdrawal validation (`shielded_withdrawal/transform_into_action/v0/mod.rs`) rejects on, among others:
- `validate_minimum_pool_notes` (anonymity floor),
- `InvalidShieldedProofError` (Orchard proof fails),
- **`validate_anchor_exists`** — the transition's `anchor` (commitment-tree Merkle root at build time) must exist in Platform's recorded-anchors tree,
- `validate_nullifiers` (double-spend / intra-bundle dup).

The wallet computes the withdrawal anchor **locally** (`operations.rs::extract_spends_and_anchor`): `store.witness(note.position).root(cmx)` — the root of the wallet's own commitment tree at the note's checkpoint. Platform accepts it only if that exact root is one it recorded. A wallet whose local tree diverges from any Platform-recorded checkpoint (sync lag, frontier discrepancy, or a note in the local tree Platform hasn't recorded) produces an anchor Platform never had → **rejected every attempt**. This fits the report precisely: repeatable, never lands, funds untouched (the ST fails, notes aren't spent), and — because the rejection apparently doesn't reach the wallet as a clean consensus verdict — it lands in the ambiguous `ShieldedSpendUnconfirmed` bucket ("may have gone through") instead of a clear failure.

**Why it may not surface as a clear failure:** if the rejection is delivered as a `StateTransitionBroadcastError` with empty consensus data (or the result is never retrievable because the ST is refused pre-block), `classify_spend_wait_failure` / `broadcast_definitely_failed` treat it as ambiguous → unconfirmed.

### Code-dive conclusion (high confidence)

The anchor the wallet submits is **not guaranteed to be a Platform-recorded anchor**, by construction:

| Side | Behaviour |
|---|---|
| **Wallet anchor** | `extract_spends_and_anchor` → `witness(position, 0)` — **depth 0 = the current tree root** (matches the proof's `tree_anchor()`, also depth 0). |
| **Wallet sync** | appends commitments in `CHUNK_SIZE` units (`aligned_start = already_have / CHUNK_SIZE * CHUNK_SIZE`) and checkpoints at the post-append leaf count — **aligned to chunk/stream boundaries, never to block boundaries**. |
| **drive recording** | `record_anchor_if_changed` runs at **block-processing-end** — exactly **one anchor per block** (the block-end root), retained `shielded_anchor_retention_blocks = 1000`. |
| **drive validation** | `validate_anchor_exists` → `has_shielded_anchor(anchor)`; absent → **`InvalidAnchorError`**. |

So the wallet's depth-0 root equals a drive-recorded anchor **only** when its current tree happens to sit exactly on a block-end commitment count. Any other state — mid-block stream stop, commitments appended past the last block drive recorded, or a `CHUNK_SIZE` boundary that isn't a block boundary — yields a root **Platform never recorded** → every withdrawal rejected. The developers already flag this exact hazard at `sync.rs:548` (*"the depth-0 witness then reflects a state Platform never recorded"*) and fixed only the checkpoint-id-dedup variant; the **block-alignment gap remains unguarded**. This matches the report precisely (repeatable, never lands, funds untouched) and is consistent with the rejection arriving ambiguously enough to be misclassified as `ShieldedSpendUnconfirmed`.

Note: shield (deposit) does **not** spend, so it carries no anchor — which is why the user could fund a shielded balance yet never complete a spend. The same depth-0 anchor is used by `unshield` and `shielded_transfer`, so those are expected to be equally affected.

### Fix direction (B)
Build the spend/withdrawal proof against a **recorded** anchor, not the bleeding-edge depth-0 root: select the most-recent checkpoint whose root drive actually recorded (a block-aligned, within-1000-block root) and generate the witness against **that** checkpoint. This is the standard shielded-wallet pattern (spend against a confirmed anchor, not the tip). Requires the wallet to know which of its checkpoints are block-aligned/recorded — i.e., associate checkpoints with the block heights drive records anchors at.

### Still worth a testnet datapoint
A single testnet `tracing=debug` withdrawal capturing `InvalidAnchorError` (vs. a different rejection) converts "high confidence" to "confirmed" before investing in the fix. Cheap if a shielded-funded testnet wallet already exists; otherwise it is the multi-step bootstrap.

### Separable bug (overlaps with A)
A definitively-rejected withdrawal is **misclassified** — it should surface as a clear failure (release the reservation, allow retry), not "may have gone through." That is the **A** reservation-release / classification work.

## Reproduction — hard evidence (deterministic, no testnet)

Two committed tests reproduce the root cause end to end, without a testnet:

1. **Wallet produces a non-recorded anchor** — `platform-wallet` (`--features shielded`), `wallet::shielded::file_store::tests::depth0_spend_anchor_mid_block_is_not_a_recorded_block_boundary_anchor` — **PASSES**. On the real SQLite-backed commitment tree it appends two "blocks" of commitments, captures the depth-0 `tree_anchor()` at each block boundary (what drive records), then stops **mid-block** (index-chunk sync) and shows the wallet's mid-block anchor is **neither** recorded boundary anchor. It also pins that the spend anchor `witness(0).root(cmx)` equals that mid-block `tree_anchor()` — i.e. the value the withdrawal actually submits.

2. **drive rejects that anchor** — `drive-abci` (`--features shielded_test_data`), `…shielded_withdrawal::tests::…::test_valid_proof_with_unrecorded_anchor_returns_invalid_anchor_error` — a **real** Orchard proof, correct value balance, sufficient pool, but the anchor is not recorded (no `insert_anchor_into_state`) → `StateError::InvalidAnchorError`. Identical to `test_valid_shielded_withdrawal_proof_succeeds` except for the missing anchor record, isolating the anchor as the sole cause.

Chain: **(1)** the wallet submits a mid-block anchor drive never recorded; **(2)** drive rejects exactly that with `InvalidAnchorError`; the proof passes first (proof-before-anchor order), so the failure isn't a proof/structure problem — it's the anchor. This is the user's "withdrawal never lands," confirmed in code.

## UX follow-ups (independent of the root cause)

- Don't title the ambiguous outcome "Success." Use a neutral "Submitted — confirming" state.
- After the next sync reconciles, surface the **definitive** outcome (landed vs returned-to-spendable) instead of leaving the user guessing.
- Consider releasing the reservation on the reconcile pass (not only on restart) so funds free up without a relaunch.
