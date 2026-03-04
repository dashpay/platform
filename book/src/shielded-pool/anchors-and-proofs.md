# Anchors and Spend Proofs

This chapter explains why anchors -- Merkle roots of the commitment tree -- are the
mechanism that makes shielded spend proofs work, and how the platform records and
validates them.

## The Core Problem

When a user spends a shielded note, they must prove to the network that the note
exists without revealing *which* note it is. This is the fundamental tension of a
privacy system: the network needs to be convinced that value is real, but the spender
needs to hide which specific entry they are spending.

## What Is an Anchor?

An **anchor** is the root hash of the Sinsemilla Merkle tree at a particular block
height. The platform maintains a commitment tree -- a binary Merkle tree of depth 32,
hashed with the Sinsemilla hash function -- that holds every note commitment (`cmx`)
ever created in the shielded pool. Each time a new note is created (via Shield,
ShieldedTransfer, or ShieldFromAssetLock), its `cmx` is appended to this tree.

At the end of each block, after all state transitions have been applied, the platform
computes the current root hash (anchor) of the commitment tree. If the anchor changed
from the previous block (i.e., new notes were added), it is recorded on-chain:

```
Anchors tree: block_height (8 bytes BE) --> anchor_bytes (32 bytes)
```

The anchors tree lives at `[AddressBalances, "s", [6]]` -- a subtree of the shielded
credit pool.

## Why Anchors Make Spend Proofs Possible

A spend proof is a zero-knowledge proof that says:

> "I know a note with value V at position P in the commitment tree, and I have the
> spending key that controls it. The tree root at the time I am referencing was A."

The proof reveals *none* of those details to the verifier. It only reveals:

1. A **nullifier** (a deterministic, unique tag derived from the note and spending
   key). The nullifier prevents double-spending -- once published, the same note
   cannot be spent again.
2. The **anchor** (A) that the proof was computed against.

### The Trust Chain

The anchor creates a chain of trust between the spender's private knowledge and the
public state:

```
Note exists in tree  <--  Merkle witness proves inclusion  <--  Anchor binds to state
       (private)              (inside ZK proof)                    (public, on-chain)
```

Step by step:

1. **The spender knows a note at position P.** They have the note's value, recipient
   address, randomness (rseed), and the spending key. This is all private.

2. **The spender generates a Merkle witness.** A Merkle witness (authentication path)
   is a sequence of 32 sibling hashes from the leaf at position P up to the root.
   Given the leaf and the witness, anyone can recompute the root. The witness proves
   that a specific leaf is part of the tree *at the root the witness was generated for*.

3. **The spender builds a ZK proof.** The Halo 2 circuit takes as private inputs:
   - The note (value, address, rseed, rho)
   - The spending key
   - The Merkle witness (32 sibling hashes)

   And as public inputs:
   - The nullifier (derived deterministically from the note and key)
   - The anchor (the root hash the witness was computed against)

   The circuit verifies internally that:
   - The note commitment `cmx` is correctly derived from the note fields
   - The Merkle witness is a valid path from `cmx` to the declared anchor
   - The nullifier is correctly derived from the note and spending key
   - The spending key matches the note's recipient address

   If all checks pass, the proof is valid. The verifier (the platform) learns the
   nullifier and anchor, but nothing else.

4. **The platform validates the anchor.** The platform checks that the anchor in the
   proof matches a historical anchor that was actually recorded on-chain. This is
   critical -- without this check, a spender could fabricate a commitment tree that
   contains their fake note and produce a valid proof against that fake root. The
   anchor check ties the proof to the *real* tree state.

5. **The platform checks the nullifier.** If the nullifier has been seen before, the
   spend is rejected (double-spend attempt). If it is new, it is recorded in the
   nullifier tree to prevent future reuse.

### Why This Preserves Privacy

The anchor is a single 32-byte hash that represents the entire state of the
commitment tree at a point in time. Many notes share the same anchor (every note
that existed at that block height). The platform learns that the spender's note is
*somewhere* in the tree at that block height, but not *where* -- the tree could
contain millions of notes, and the proof reveals no information about position.

Furthermore, the nullifier is deterministic but unlinkable to the note commitment.
Given a nullifier, you cannot determine which `cmx` it corresponds to without
knowing the spending key. This means:

- You cannot link a spend to the transaction that created the note
- You cannot determine which of the millions of notes in the tree was spent
- You cannot even determine whether two spends came from the same wallet
  (different notes produce different nullifiers, even from the same key)

The only information leaked is the anchor's block height, which reveals an
upper bound on when the note was created. Using an older anchor widens the
anonymity set (more notes existed at that point), while using a very recent anchor
narrows it slightly. Clients should use recent-but-not-latest anchors for a good
balance of privacy and liveness.

## The Full Lifecycle

```
1. Shield:  Client deposits credits, platform appends cmx to commitment tree
                                              |
2. Block end:  Platform computes new anchor = root(commitment_tree)
               If changed, stores (block_height -> anchor) in anchors tree
                                              |
3. Sync:    Client fetches notes, appends all cmx values to local tree
            Client checkpoints at each block
                                              |
4. Spend:   Client picks a note and its position in the local tree
            Client picks a historical anchor (must exist on-chain)
            Client generates Merkle witness at that anchor's checkpoint
            Client builds ZK proof with (note, witness, anchor) as inputs
                                              |
5. Verify:  Platform receives the state transition containing the proof
            Platform checks: anchor in proof matches a recorded on-chain anchor
            Platform checks: nullifier has not been seen before
            Platform verifies: Halo 2 proof is valid
            Platform records: nullifier in nullifier tree (prevents reuse)
            Platform updates: pool balance (deducts value_balance)
```

## Why Historical Anchors Are Necessary

The platform does not require spenders to use the *latest* anchor. Any anchor that
was ever recorded on-chain is valid. This is essential for two reasons:

**1. Concurrency.** Between the time a client builds a proof and the time the
platform processes it, other transactions may have added notes to the tree, changing
the anchor. If only the latest anchor were valid, proofs would become stale almost
immediately.

**2. Privacy.** If all spenders were forced to use the latest anchor, it would
reveal that their note was created before that block. With historical anchors, a
spender can reference an anchor from any past block, making it impossible to narrow
down when the note was created beyond "sometime before block N" where N can be
any block the spender chooses.

The platform simply checks that the submitted anchor exists in the anchors tree.
There is no requirement for recency beyond the fact that the anchor must correspond
to a real state of the commitment tree.

## Anchor Recording: Implementation

The anchor recording happens in `record_shielded_pool_anchor_if_changed`, which
runs at the end of each block proposal (after all state transitions have been
processed):

1. **Read the current anchor** from the CommitmentTree at
   `[AddressBalances, "s", [1]]` using `commitment_tree_anchor()`.

2. **Query the most recent stored anchor** from the anchors tree at
   `[AddressBalances, "s", [6]]` (descending query, limit 1).

3. **Compare.** If the current anchor differs from the latest stored anchor
   (or no anchor has been stored yet and the tree is non-empty), store the new
   anchor keyed by block height.

This is a post-processing step, not a per-transaction step. Even if multiple
shielded transactions in the same block add notes, only one anchor is recorded
for the entire block. This keeps the anchors tree compact.

### Version Gating

Anchor recording uses the standard `OptionalFeatureVersion` dispatch pattern:

```rust
match platform_version.drive_abci.methods.block_end.record_shielded_pool_anchor {
    None => Ok(()),      // Protocol versions before v12 -- no shielded pool
    Some(0) => self.record_shielded_pool_anchor_if_changed_v0(...),
    Some(v) => Err(UnknownVersionMismatch { ... }),
}
```

Protocol versions 1--11 have this field set to `None`, so the function is a no-op.
Protocol version 12 (which introduces the shielded pool) sets it to `Some(0)`.

## Client-Side: Generating Witnesses

The client maintains a `ClientCommitmentTree` -- a local mirror of the on-chain
Sinsemilla tree. As the client syncs notes from the platform:

1. Every `cmx` encountered is appended to the local tree (marked as `Retention::Marked`
   for the client's own notes, `Retention::Ephemeral` for others).
2. After processing each block's notes, the client calls `tree.checkpoint(block_height)`.
3. To spend a note at position P, the client calls `tree.witness(position, 0)` to
   obtain a `MerklePath` -- the 32-sibling authentication path.
4. The client calls `tree.anchor()` to get the current root hash, which must match
   a historical anchor on the platform.

The `ClientCommitmentTree` retains enough internal state to produce witnesses at any
checkpoint it has stored (up to its configured retention limit). This allows the
client to generate witnesses against past anchors, not just the latest one.

## Security Properties

| Property | How Anchors Help |
|---|---|
| **Soundness** | A proof against anchor A is only valid if the note actually exists in the tree at state A. A fake note would require finding a Sinsemilla hash collision. |
| **Privacy** | The anchor reveals only "the note existed at or before block N". The anonymity set is every note in the tree at that block. |
| **Double-spend prevention** | The nullifier (not the anchor) prevents double-spending. The anchor proves the note *exists*; the nullifier ensures it is spent only *once*. |
| **Liveness** | Historical anchors remain valid indefinitely, so proofs never expire due to tree state changes. |
| **Binding** | The anchor is included in the Halo 2 public inputs and in the bundle commitment. Changing the anchor after proof generation invalidates the proof. |

## Relationship to Other Components

- **Fees:** The fee is encoded in `value_balance` and bound to the proof via the
  binding signature. See [Shielded Transaction Fees](../fees/shielded-fees.md).
- **Return proofs:** Platform return proofs for shielded transitions prove the
  aggregate pool balance changed, not individual notes. See
  [Return Proofs](../state-transitions/return-proofs.md).
- **Light client sync:** Clients fetch historical anchors via the `GetShieldedAnchors`
  gRPC query to verify their local tree state matches the platform. See the
  [Client Integration Guide](../../docs/SHIELDED_CLIENT_INTEGRATION.md).
