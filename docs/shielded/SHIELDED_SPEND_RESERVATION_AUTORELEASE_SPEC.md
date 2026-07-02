# Spec — auto-release a stranded shielded-spend reservation on sync (bug A)

## Problem
A shielded spend that returns `ShieldedSpendUnconfirmed` (broadcast accepted, but
the result-wait failed ambiguously) intentionally **keeps** the spent notes'
reservation (`SubwalletState.pending_nullifiers`) so a retry can't double-spend.
That reservation is released only when:
- the tx **lands** (a later sync sees the note spent → `mark_spent` clears it), or
- the app **restarts** (`pending_nullifiers` is in-memory, never persisted).

So a spend that is broadcast-accepted but **never lands** (lost ACK / mempool
eviction) strands its notes for the whole session — the funds look stuck, and the
UI says "may have gone through — do not retry" with no in-session resolution.

Bug **B** (PR #3977) already removed the *dominant* trigger (anchor-mismatch
spends are now refused pre-broadcast with the notes released), so A is the
**residual** case. This fix closes it.

## Chosen approach: release when the spend's anchor is provably pruned
When a spend reaches `ShieldedSpendUnconfirmed`, remember the **recorded anchor**
it was built against (post-B, always a Platform-recorded root). On each shielded
sync, after the normal spent-note reconcile, release any still-pending reservation
whose anchor is **no longer in Platform's recorded anchor set**.

Rationale (fund-safe, no double-spend): Platform's `validate_anchor_exists` accepts
a shielded spend only while its anchor is retained (`shielded_anchor_retention_blocks
= 1000`). Once the anchor is pruned, the transition can **never** execute — so if
the nullifier is also still unspent (which "still pending after the sync reconcile"
already implies), the spend is provably dead and its notes can be freed. The
authoritative double-spend guard remains the on-chain nullifier set.

Reuses B's `GetShieldedAnchors` query (`ShieldedAnchors::fetch_current`), so no new
query type and no protocol change. "Anchor pruned" is the same ~1000-block bound as
a height window, but detected directly (no height arithmetic / new watermark).

## Data model
`SubwalletState.pending_nullifiers: BTreeSet<[u8;32]>` →
`BTreeMap<[u8;32], PendingSpend>` where
```
struct PendingSpend { anchor: Option<[u8;32]>, activity_id: Option<[u8;32]> }
```
- `mark_pending(nullifier)` inserts `{ anchor: None, activity_id: None }` (a
  just-reserved, not-yet-built spend).
- A new `set_pending_spend(nullifier, anchor, activity_id)` fills them in once the
  spend is built (anchor known, activity row created).
- `clear_pending` / `mark_spent` remove the entry (unchanged semantics).
- `unspent_notes` filter: `!pending_nullifiers.contains_key(&n.nullifier)`.
In-memory only, as today (not persisted → a restart still frees everything).

## Data flow
- **Spend paths** (`unshield`, `shielded_transfer`, `withdraw`,
  `identity_create_from_shielded_pool`): after `extract_spends_and_anchor` returns
  the anchor and the `record_pending_activity` entry exists, call
  `set_pending_spend(nullifier, anchor, activity_id)` for each selected note —
  inside the async block, before `broadcast_shielded_spend`. A definite-failure
  path (`cancel_pending`) still removes the entry; a `ShieldedSpendUnconfirmed`
  keeps it (now carrying the anchor).
- **Sync reconcile** (`coordinator.rs`, right after the `note.is_spent →
  mark_spent` loop): collect still-pending `(nullifier, anchor)` pairs across the
  synced subwallets; if any exist, fetch the recorded set once
  (`ShieldedAnchors::fetch_current`); for each pair whose `anchor ∉ recorded`,
  `clear_pending(nullifier)` **and** `record_activity_status(activity_id, Failed)`
  so the UI shows a clear, retryable failure instead of "Pending" forever.
  Skip the fetch entirely when no anchored reservations exist (the common case).

## Fund safety
- Release only when `anchor ∉ recorded` (pruned) → the spend can never execute →
  no double-spend. The nullifier set stays authoritative.
- Never releases a still-valid (recorded-anchor) reservation, so a slow-but-landing
  tx is not re-spendable while it could still confirm.
- `None`-anchor entries (reserved but not yet built) are transient and handled by
  the existing error paths; the sync release ignores them.

## Test plan (Rust, `--features shielded`)
- **Store unit:** a reservation with a recorded anchor survives a sync where the
  anchor is still recorded; is released when the anchor is absent from the
  recorded set; a reservation with a still-recorded anchor is NOT released.
- **`unspent_notes`** excludes a pending nullifier and re-includes it after release.
- **Activity:** on release the linked activity flips `Pending → Failed`.
- Full `platform-wallet --features shielded` suite green; fmt + clippy; iOS build.

## Scope / not in scope
- **This PR:** the reservation auto-release only. No protocol/DAPI change.
- **Not:** the `ShieldedSpendUnconfirmed` message wording — it is honest for this
  residual (genuinely-ambiguous) case now that B handles the anchor-mismatch case.
  A softer "Pending — funds free automatically if it doesn't confirm" is an
  optional Swift follow-up.
- Stacks on #3977 (B); the anchor stored must be a recorded one, which only B
  guarantees.
