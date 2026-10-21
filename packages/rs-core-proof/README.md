# dash-core-proof

Verifies Dash Core quorum proof chains (`DASHNC02`, [DIP 175](https://github.com/dashpay/dips/pull/175)),
the format Dash Core's `getquorumproofchain` RPC returns.

A client that trusts a checkpoint (a block's height and hash and the masternode and quorum
Merkle roots its coinbase commits to) can check a later block's roots, and any quorum
commitment or masternode entry under them, without trusting whoever served the proof:

```rust
use dash_core_proof::{bootstrap, parse_commitment, State};

fn quorum_key(envelope: &[u8], checkpoint: &State, min_height: u32) -> dash_core_proof::Result<[u8; 48]> {
    let verified = bootstrap::verify(envelope, checkpoint, min_height)?;
    // Check that the record is the quorum you asked for before using it.
    let commitment = parse_commitment(verified.records()[0].leaf)?;
    Ok(commitment.public_key)
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
  64 bytes, with no trailing bytes. The one place it is stricter than Core, BLS signature bytes
  Core would read as null, cannot occur in a valid block. Every length is bounded before
  allocation.

## Trust model

The checkpoint, and that no ChainLock quorum active at the checkpoint or mined after it ever
signs dishonestly, including quorums long since retired: the relay picks the path, and like
Core's verifier this one does not check that a signer was the quorum selected for that height.
It does not replay DKG or re-derive which members signed. An older checkpoint therefore trusts
more quorums for longer; refresh checkpoints with releases.

A valid proof does not show that its target is the newest ChainLock; how fresh a target must
be is the caller's policy.

## Tests

`cargo test -p dash-core-proof`. The fixtures in `tests/data` are unmodified
`getquorumproofchain` results from mainnet and testnet nodes.
