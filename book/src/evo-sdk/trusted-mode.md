# Trusted Mode and Proof Verification

## The problem

Every Platform query response includes a cryptographic proof — a signed hash
from the current validator quorum that attests the response matches the
platform state tree. To verify these proofs the SDK needs the **quorum public
keys** for the active validator set.

On a full node you can look up quorum keys directly from the Core chain. In a
browser or lightweight environment you cannot. Trusted mode solves this.

## How trusted mode works

When you create an SDK with `trusted: true`, the `connect()` call does an extra
step before returning: it fetches the current quorum public keys from a
well-known HTTPS endpoint and caches them in memory.

```typescript
const sdk = EvoSDK.testnetTrusted();
await sdk.connect(); // fetches quorum keys, then connects to DAPI
```

The trust model:

- You trust the HTTPS endpoint (operated by Dash Core Group) to return correct
  quorum public keys.
- Once the keys are cached, every subsequent query response is verified against
  them — you do **not** trust individual DAPI nodes for the data itself.

This is a pragmatic trade-off: you trust one endpoint for the validator set,
but verify all actual data cryptographically.

## When to use trusted mode

| Scenario | Trusted mode? | Why |
|----------|:---:|-----|
| Browser app | Yes | No access to Core chain |
| Node.js script | Yes | Simplest setup |
| Server with Core RPC | Optional | Can fetch quorum keys from your own node |
| Local Docker setup | Yes | Use `EvoSDK.localTrusted()` |

If you do not use trusted mode, the SDK still works but cannot verify proofs.
Queries return data but you are trusting the responding DAPI node.

## Proofs in responses

By default, the SDK verifies proofs internally and returns just the data. To
inspect the proof metadata yourself, use the `WithProof` variants:

```typescript
// Standard — proof verified internally, returns data only
const identity = await sdk.identities.fetch(id);

// With proof — returns both data and proof metadata
const { data, proof, metadata } = await sdk.identities.fetchWithProof(id);
console.log('Block height:', metadata.height);
console.log('Core chain locked height:', metadata.coreChainLockedHeight);
```

Some methods also offer an `Unproved` variant that skips proof verification
entirely, useful when you trust the node or want faster responses:

```typescript
const identity = await sdk.identities.fetchUnproved(id);
```
