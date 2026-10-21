# dash-core-proof

Verifies Dash Core quorum proof chains (`DASHNC02`, [DIP 175](https://github.com/dashpay/dips/pull/175)),
the format Dash Core's `getquorumproofchain` RPC returns.

A client that trusts a checkpoint (a block's height and hash and the masternode and quorum
Merkle roots its coinbase commits to) can check a later block's roots, and any quorum
commitment or masternode entry under them, without trusting whoever served the proof:

```rust
use dash_core_proof::bootstrap::{self, RecordKind};
use dash_core_proof::{parse_commitment, Hash, State};

/// The key of quorum (`quorum_type`, `quorum_hash`, in wire order), if the
/// envelope proves it from `checkpoint` at `min_height` or later.
fn quorum_key(
    envelope: &[u8],
    checkpoint: &State,
    min_height: u32,
    quorum_type: u8,
    quorum_hash: &Hash,
) -> Option<[u8; 48]> {
    let verified = bootstrap::verify(envelope, checkpoint, min_height).ok()?;
    // Every record is authentic, but the relay chose which ones to open:
    // use only the quorum that was asked for.
    verified
        .records()
        .iter()
        .filter(|record| record.kind == RecordKind::Quorum)
        .filter_map(|record| parse_commitment(record.leaf).ok())
        .find(|c| c.quorum_type == quorum_type && c.quorum_hash == *quorum_hash)
        .map(|c| c.public_key)
}
```

A `State` deserializes from the same JSON Core's RPC uses for `target`, so a block's
`getquorumproofchain` target can serve as a checkpoint file.

## What it checks

- The chain starts at exactly the checkpoint the caller supplies.
- Each ChainLock certificate is a valid Basic-scheme BLS signature by the current ChainLock
  quorum, at a strictly increasing height.
- Each next ChainLock quorum's commitment is mined in the certified block, or in an ancestor
  linked to it header by header.
- The target block's coinbase (payload version 3, or 4 from Core v24) commits to the returned
  roots, and every record opens under them.
- Nothing Core's verifier rejects is accepted: CompactSizes, commitments and BLS points must be
  canonical, Merkle paths free of duplicate-leaf ambiguity, leaves and transactions other than
  64 bytes, with no trailing bytes. The one place it is stricter than Core is legacy-scheme BLS
  signature bytes Core would read as null; in a coinbase's best ChainLock signature those are
  valid only before any ChainLock, and a proof ending at such a block is refused. Every length is
  bounded before allocation.

## Trust model

The checkpoint, and that no ChainLock quorum mined less than 4,096 blocks before the checkpoint,
or at any time after it, ever signs dishonestly, including quorums long since retired. The relay
picks the path: a link's ancestor headers (at most 4,096 per proof) can reach a quorum mined
before the checkpoint, even one retired by then, and like Core's verifier this one does not
check that a signer was the quorum selected for that height.
It does not replay DKG or re-derive which members signed. An older checkpoint therefore trusts
more quorums for longer; refresh checkpoints with releases.

A valid proof does not show that its target is the newest ChainLock; how fresh a target must
be is the caller's policy.

## Tests

`cargo test -p dash-core-proof`. The fixtures in `tests/data` are unmodified
`getquorumproofchain` results from mainnet and testnet nodes.
