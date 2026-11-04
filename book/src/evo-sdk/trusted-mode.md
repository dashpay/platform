# Trusted Mode, Verified Mode and Proof Verification

## The problem

Every Platform query response includes a cryptographic proof — a signed hash
from the current validator quorum that attests the response matches the
platform state tree. To verify these proofs the SDK needs the **quorum public
keys** for the active validator set.

On a full node you can look up quorum keys directly from the Core chain. In a
browser or lightweight environment you cannot. Trusted mode and verified mode
are the two ways the SDK gets them there.

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

## How verified mode works

On mainnet and testnet, `verified: true` removes that trust. The SDK proves
each quorum public key with a DIP 175 quorum proof, starting from a checkpoint
built into the SDK. The quorum service only relays the proofs: a relay that
lies can delay requests, but it cannot make the SDK accept a forged key.

```typescript
const sdk = EvoSDK.testnetVerified();
await sdk.connect(); // fetches no quorum keys up front
```

Nothing is fetched at `connect()`. A key is proven the first time the SDK
verifies a response signed by its quorum, so the first query after a quorum
rotation takes a little longer. In the browser, proof verification yields to
the event loop between links, so the page stays responsive.

Proofs grow with the age of the state they start from. A long-lived app should
save `sdk.verifiedContinuation()` (for example on page hide) under a key per
network and pass it back as `continuation` on the next start. If `connect()`
refuses a saved continuation, drop it and start from the built-in checkpoint.
The [Evo SDK README](https://github.com/dashpay/platform/tree/master/packages/js-evo-sdk#verified-quorum-keys)
shows the full pattern.

Verified mode needs relays that serve proofs. The public mainnet quorum service
does not serve them yet, so on mainnet pass `quorumRelays` that do. Verified
mode is not available on local networks or devnets, and it requires proofs
(`proofs: false` is refused).

## Which mode to use

| Scenario | Mode | Why |
|----------|------|-----|
| Browser app on testnet | Verified | No access to the Core chain, and no endpoint to trust |
| Browser app on mainnet | Verified with your own `quorumRelays`, or trusted | The public mainnet service does not serve proofs yet |
| Node.js script | Verified or trusted | Trusted is the simplest setup |
| Server with Core RPC | Optional | Can fetch quorum keys from your own node |
| Local Docker setup or devnet | Trusted | Use `EvoSDK.localTrusted()`; verified mode needs mainnet or testnet |

Trusted mode stays the default. If you use neither, the SDK still works but
cannot verify proofs: queries return data, but you are trusting the responding
DAPI node.

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
