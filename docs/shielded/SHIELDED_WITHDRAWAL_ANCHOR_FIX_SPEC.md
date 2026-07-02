# Spec — shielded spend uses a Platform-recorded anchor

Fixes the reproduced root cause of TestFlight report B (see
`TESTFLIGHT_FEEDBACK_INVESTIGATION.md`): shielded spends (withdraw / unshield /
transfer) are rejected with `InvalidAnchorError` because the wallet builds the
proof against its **depth-0** commitment-tree root, which — with an index-chunk
sync (`CHUNK_SIZE = 2048`) that routinely stops mid-block — is a root Platform
never recorded (drive records **one anchor per block**, retains 1000).

Reproduced by two passing tests (committed `6959fe967e`):
`platform-wallet …depth0_spend_anchor_mid_block_is_not_a_recorded_block_boundary_anchor`
and `drive-abci …test_valid_proof_with_unrecorded_anchor_returns_invalid_anchor_error`.

## Goal / non-goal

- **Goal:** a shielded spend the wallet builds is accepted by Platform's
  `validate_anchor_exists` — i.e. its anchor is always a drive-recorded root.
- **Goal:** when the wallet cannot yet build against a recorded anchor, fail
  with a **clear, retryable** error instead of broadcasting a doomed transition
  that surfaces as "may have gone through."
- **Non-goal (this spec):** the reservation-release / `ShieldedSpendUnconfirmed`
  misclassification (tracked as "A"); UX copy. Called out where they interact.

> **Revised after 3-reviewer audit** (soundness / feasibility / scope). The
> cryptographic core is confirmed sound; changes folded in: drop the
> most-recent-anchor fast-path; `anchor_at_depth` is an explicit store-trait
> addition (implemented via the `witness(marked_pos, d).root(cmx)` workaround to
> avoid an external `grovedb-commitment-tree` crate change); position-aware note
> selection is required (B1); the "no recorded checkpoint" outcome (B2) is a
> graceful, fund-safe retryable error — auto-enable in that pathological case is
> a follow-up (see Scope).

## Chosen approach: build against the most-recent recorded anchor

Instead of `witness(pos, 0)` (bleeding-edge root), select the **shallowest
checkpoint whose root Platform has recorded**, and witness + prove against that.

Data flow, per spend (`extract_spends_and_anchor`):

1. **Fetch the recorded anchor set** from Platform: `ShieldedAnchors::fetch_current(sdk)`
   (`ShieldedAnchors(Vec<[u8;32]>)`) — the ≤1000 retained roots, into a `HashSet`.
   (Always the full set — `GetMostRecentShieldedAnchor` would *not* match in the
   target mid-block case, since the wallet's recorded checkpoint is a *prior*
   block, not the latest.)
2. **Walk the wallet's checkpoints newest→oldest**, `d = 0..max_checkpoints`. For
   each `d`, compute the tree root at `d` (`anchor_at_depth(d)`); take the first
   `d` whose root ∈ the recorded set — `d*` / `anchor*`, with tree size
   `size* = checkpoint_id@d*`. `d = 0` is the fully-synced fast path.
3. **Select notes confined to `d*`:** filter candidate `unspent_notes` to
   `position < size*` *before* value-based coverage selection, so every selected
   note is witnessable at `d*`.
4. **Witness every selected note at `d*`** (`witness(pos, d*)`), assert each
   witness root equals `anchor*`, and pass `anchor*` to the proof builder.
5. **Failure → retryable, no broadcast:**
   - No `d` in `0..max_checkpoints` has a recorded root → `ShieldedNoRecordedAnchor`.
   - A `d*` exists but confirmed-at-`d*` balance (notes with `position < size*`)
     can't cover amount+fee → `ShieldedNoRecordedAnchor` (same class: "wait for
     the next sync"). Never surface `ShieldedMerkleWitnessUnavailable` here.

Note requirement (B1): value-based selection alone would pick a note newer than
`d*`, whose `witness(pos, d*)` returns `Err(NotContained)` (→ opaque
`ShieldedMerkleWitnessUnavailable`). The `position < size*` pre-filter prevents
this; deeper `d` only has *fewer* eligible notes, so if the shallowest recorded
`d*` can't cover the amount, no deeper one can — fail fast with the retryable
error.

### Why this approach
- **Correct by construction:** the anchor is a value Platform recorded, so
  `validate_anchor_exists` passes; the drive repro test's rejection cannot occur.
- **Uses existing primitives:** `GetShieldedAnchors` query + `witness(pos, depth)`
  already exist; the proof builder already takes an explicit `anchor`.
- **No stream/tree-format change:** avoids block-aligned checkpointing, which is
  infeasible because the sync stream exposes only a per-chunk `block_height`, not
  per-commitment block boundaries.
- **Fund-safe:** spending against an older recorded anchor is standard shielded
  practice; the note is unspent, the witness root equals the anchor, and the
  nullifier set is the authoritative double-spend guard.

## Alternatives rejected
- **Block-aligned checkpointing** (checkpoint at each block's last commitment):
  cleanest in theory, but the sync stream gives only per-chunk `block_height`
  (chain tip at response time), not per-commitment block membership — the wallet
  cannot identify block boundaries within a chunk. Rejected as infeasible without
  a drive/DAPI query-shape change.
- **Precondition check only** (verify depth-0 anchor is recorded; else clear
  error, don't broadcast): safe and small, but does **not** enable withdrawal
  when mid-block — leaves the user unable to withdraw. Kept as the *fallback*
  branch (step 4), not the whole fix.
- **Depth-`N`-back heuristic** (always spend `N` checkpoints back): fragile — `N`
  checkpoints is not a fixed number of blocks, and a chunk-boundary checkpoint
  root still isn't guaranteed recorded. Rejected.

## Interface / data-flow changes
- `ShieldedStore` trait: a single new method
  `witness_at_depth(&self, position, depth) -> Result<Option<MerklePath>, Err>`;
  the existing `witness` becomes a default delegating to `witness_at_depth(pos, 0)`
  (so existing callers are unchanged). Implemented by `FileBackedShieldedStore`
  (passes `depth` to `ClientPersistentCommitmentTree::witness`) and
  `InMemoryShieldedStore` (same stub as before, `depth`-agnostic). No `dyn`/FFI
  implementor exists, so the addition is contained.
  - *As-built note:* the anchor at a depth is derived **inline** from the
    selected notes' own witnesses (`witness_at_depth(note.position, depth).root(cmx)`),
    so no separate `anchor_at_depth` was needed. The "note too new for this
    checkpoint" case is detected directly by `witness_at_depth` returning
    `Ok(None)`/`Err` at that depth (→ the probe stops), so no
    `checkpoint_id_at_depth` / explicit `position < size*` filter is needed
    either — both were considered in earlier drafts but the localized probe
    subsumes them.
- `extract_spends_and_anchor` gains `sdk: &Arc<Sdk>` and does the fetch +
  probe (via the pure, unit-testable `select_recorded_spends`). It has FOUR
  callers: `withdraw`, `unshield`, `shielded_transfer`,
  `identity_create_from_shielded_pool`.
- New retryable error `PlatformWalletError::ShieldedNoRecordedAnchor`; FFI/UI maps
  it to "still syncing — try again shortly" (distinct from `ShieldedSpendUnconfirmed`).
- Query: `ShieldedAnchors::fetch_current(sdk).await` (existing `FetchCurrent` impl).

## Failure modes / risks
- **Anchor/witness mismatch** → proof rejected. Mitigation: assert each selected
  note's `witness(pos, d*).root(cmx)` equals `anchor*` (extends today's "all notes
  agree" check to equal the *selected* anchor).
- **Concurrency (C1):** depth indices shift if a sync checkpoints concurrently.
  Mitigation: fetch the anchor set *outside* the store lock; then do the
  probe (`anchor_at_depth`) **and** all note witnesses under a **single** store
  read-lock hold so `d*`, `size*`, and the witnesses are mutually consistent.
- **Recorded set (≤1000)** — `HashSet` membership; one round trip per spend (rare).
- **Pruning race** — a freshly-queried anchor has ~1000 blocks of runway; a deep
  `d*` erodes it but stays within the window at query time. Negligible.
- **No recorded checkpoint / insufficient confirmed-at-`d*` funds** → clean
  `ShieldedNoRecordedAnchor`; before broadcast, so the generic `Err` arm runs
  `cancel_pending` (reservation released) in all four callers. No fund risk.
- **Double-spend:** unchanged — the Orchard nullifier is `(nk, ρ, ψ, cm)`-derived,
  anchor-independent; the on-chain nullifier set stays authoritative (auditor-confirmed).

## Test plan
- **Keep** the two reproduction tests (they pin the pre-fix mechanism + the drive
  contract; both remain valid).
- **Wallet unit (new):** given a tree with recorded anchors at block-boundary
  sizes {B1,B2} and a mid-block depth-0 state, `select_recorded_anchor` returns
  `B2`'s anchor (most-recent recorded), not the depth-0 root; and witnessing at
  that depth yields a root ∈ recorded set.
- **Wallet unit (new):** no checkpoint root recorded → `ShieldedNoRecordedAnchor`.
- **drive-abci (existing, real-proof):** a withdrawal built against a **recorded**
  anchor succeeds (`…_proof_succeeds` already covers this) — confirms the fix's
  output shape is accepted.
- **Regression:** full `platform-wallet --features shielded` + the shielded
  drive-abci suite green; `clippy --all-features`; `fmt --check`.

## Rollout / scope
- **This PR:** the wallet-only anchor-selection fix across all four spend paths +
  the `ShieldedNoRecordedAnchor` error + position-aware selection + tests. No
  protocol/drive/DAPI change.
- **Follow-up 1 (Report A — convergence):** ensure a shielded sync reliably
  reaches the tip (isn't stranded mid-chunk by wallet-switch/backgrounding), so a
  recorded checkpoint is actually created. Without it, a chronically-interrupted
  wallet gets the clean `ShieldedNoRecordedAnchor` error but still can't withdraw.
- **Follow-up 2 (robust auto-enable, protocol):** have `GetShieldedAnchors` /
  `GetMostRecentShieldedAnchor` return each anchor's **tree size** (auditor's
  suggestion) so the wallet can deliberately checkpoint on a recorded boundary and
  know exactly which notes are confirmed — collapses B1+B2 but needs drive + DAPI
  + proof-verifier + SDK changes.
- **Follow-up A (classification/reservation):** the `ShieldedSpendUnconfirmed`
  "may have gone through" misclassification + no-restart reservation release.
  This fix removes the dominant *cause*; A improves the *residual* ambiguous case.
