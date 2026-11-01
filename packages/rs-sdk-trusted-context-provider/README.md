# Trusted Context Provider for Dash SDK

This crate provides a trusted HTTP-based context provider for the Dash SDK that fetches quorum information from trusted HTTP endpoints instead of requiring Core RPC access.

## Features

- Fetches quorum public keys from trusted HTTP endpoints
- Supports mainnet, testnet, and devnet networks
- Allows custom URLs for your own trusted endpoints
- LRU caching for quorum data
- Optional fallback provider for data contracts and token configurations
- Domain resolution verification during initialization

## Networks Supported

- **Mainnet**: Uses `https://quorums.mainnet.networks.dash.org/`
- **Testnet**: Uses `https://quorums.testnet.networks.dash.org/`
- **Devnet**: Uses `https://quorums.<devnet_name>.networks.dash.org/`

## Usage

### Basic Usage

```rust
use dash_sdk::{Sdk, SdkBuilder};
use dash_sdk::dapi_client::AddressList;
use dpp::dashcore::Network;
use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;
use std::num::NonZeroUsize;

// Create the trusted context provider
let context_provider = TrustedHttpContextProvider::new(
    Network::Testnet,
    None, // devnet_name - only needed for devnet
    NonZeroUsize::new(100).expect("cache size"),
)?;

// Build SDK with the trusted context provider
let sdk = SdkBuilder::new(AddressList::default())
    .with_context_provider(context_provider)
    .build()?;
```

### With Custom URL

If you want to use your own trusted HTTP endpoint instead of the default ones:

```rust
// Create provider with custom URL
let custom_provider = TrustedHttpContextProvider::new_with_url(
    Network::Testnet,
    "https://my-trusted-server.com".to_string(),
    NonZeroUsize::new(100).unwrap(),
)?;

// Build SDK with the custom provider
let sdk = SdkBuilder::new(AddressList::default())
    .with_context_provider(custom_provider)
    .build()?;
```

### With Fallback Provider

Since the trusted HTTP provider only provides quorum public keys, you may need to set a fallback provider for data contracts and token configurations:

```rust
use dash_sdk::mock::provider::GrpcContextProvider;

// Create a fallback provider that can fetch data contracts
let grpc_provider = GrpcContextProvider::new(
    None,
    "core.example.com",
    19998,
    "dashrpc",
    "password",
    NonZeroUsize::new(100).unwrap(),
    NonZeroUsize::new(100).unwrap(),
    NonZeroUsize::new(100).unwrap(),
)?;

// Create the trusted provider with fallback
let trusted_provider = TrustedHttpContextProvider::new(
    Network::Testnet,
    None,
    NonZeroUsize::new(100).unwrap(),
)?
.with_fallback_provider(grpc_provider);

// Build SDK with the trusted provider
let sdk = SdkBuilder::new(AddressList::default())
    .with_context_provider(trusted_provider)
    .build()?;
```

### With Pre-loaded Known Contracts

You can also pre-load known contracts that will be served immediately without requiring a fallback provider:

```rust
use dpp::data_contract::DataContract;

// Load your known contracts
let dpns_contract = DataContract::from_json(...)?;
let dashpay_contract = DataContract::from_json(...)?;

// Create the trusted provider with known contracts
let trusted_provider = TrustedHttpContextProvider::new(
    Network::Testnet,
    None,
    NonZeroUsize::new(100).unwrap(),
)?
.with_known_contracts(vec![dpns_contract, dashpay_contract]);

// Build SDK with the trusted provider
let sdk = SdkBuilder::new(AddressList::default())
    .with_context_provider(trusted_provider)
    .build()?;
```

## Verified Quorum Keys (opt-in)

`VerifiedHttpContextProvider` trusts no HTTP endpoint for quorum keys. It
starts from a checkpoint, a ChainLocked Core block and the masternode and
quorum Merkle roots its coinbase commits to, and accepts a Platform quorum key
only after a Core quorum proof chain (DIP 175, `DASHNC02`) verifies from that
checkpoint and opens the quorum's commitment. Relays that serve the proofs are
untrusted byte sources: a relay that lies, stalls or fails can delay requests,
but cannot make the SDK accept a forged key or ban an honest node.

```rust
use dash_sdk::SdkBuilder;
use dash_sdk::dapi_client::AddressList;
use dpp::dashcore::Network;
use rs_sdk_trusted_context_provider::VerifiedHttpContextProvider;
use std::num::NonZeroUsize;

// Embedded checkpoint, network's public quorum service as the relay.
let provider = VerifiedHttpContextProvider::new(
    Network::Testnet,
    NonZeroUsize::new(100).expect("cache size"),
)?;

let sdk = SdkBuilder::new(AddressList::default())
    .with_context_provider(provider)
    .build()?;
```

Your own checkpoint and relays (1 to 8, `https` except on loopback):

```rust
let checkpoint = VerifiedHttpContextProvider::embedded_checkpoint(Network::Testnet)?;
let provider = VerifiedHttpContextProvider::with_checkpoint(
    Network::Testnet,
    checkpoint,
    vec!["https://relay.example.org".to_string()],
    NonZeroUsize::new(100).expect("cache size"),
)?;

// Later: persist the newest verified state and start from it next time.
let state_json = serde_json::to_string(&provider.latest_verified_state())?;
```

Behaviour:

- Mainnet and testnet only. Each provider POSTs to `<source>/proofs` with the
  checkpoint hash, a height, the quorum hash and type, and `nodeCount: 0`.
- A key is returned for a Core height `H` only when verified proofs show that
  the quorum was mined by `H` and active at some height in `[H - 24, H]`:
  among the targets whose quorum roots opened the quorum, one is at or below
  `H` and one at or above `H - 24` (see the trust model below). Failures of
  any kind (timeouts, HTTP errors, oversize or invalid proofs, proofs of
  other quorums) are reported as an unavailable quorum source, so the SDK
  tries another node without banning the one that named the quorum. A
  quorum of another type than the network's Platform quorum type is
  rejected as invalid.
- Serving costs about one small proof per 24 Core blocks per quorum in use.
  The first proof of a quorum asks for the height itself; Core usually
  answers with a target exactly there, since Platform's Core height is
  ChainLocked. Past one DKG interval (24 blocks) above the highest proven
  target, the relay is usually asked for its latest ChainLock, which serves
  the next 24 heights or more. A relay that answers with a target above the
  height, because the height has no ChainLock of its own, is asked again for
  a target up to 24 blocks below it, which serves the height by itself; the
  later target is kept as evidence.
- Fetches of one quorum are shared, and requests for at most two quorums
  are in flight at once (a fetch waiting out a backoff gives its place up).
  A relay that cannot serve a quorum for two asks (the tip or the height,
  then a height 24 blocks below it) is not asked again in that fetch; a busy
  relay (503) is retried after a backoff; relays that fail otherwise cool
  down, and so does a quorum at a height whose fetch failed. A fetch gives up after 70 s.
  Redirects are refused and responses are capped at 1 MiB.
- The cache must hold at least the active Platform quorums (24 on mainnet
  and testnet), or the provider is not built; 32 or more avoids proving keys
  again just because they were evicted.
- Data contracts come from the known-contracts cache and the system contracts
  enabled by cargo features; token configurations from the known cache.
- The public mainnet quorum service currently answers `/proofs` with HTTP 403.
  `VerifiedHttpContextProvider::new(Network::Mainnet, ..)` builds, but its
  fetches fail until the service serves proofs; pass working mainnet relays
  to `with_checkpoint` meanwhile.

### Trust model

What a served key proves: the quorum was mined by the response's Core height
`H`, and active at some height in `[H - 24, H]`. Platform quorums (mainnet
type 4, testnet type 6) do not rotate: the active set at a height is the
newest quorums mined by it, so a quorum enters the set when it is mined and
never returns once it leaves. A quorum in the quorum roots of two verified
targets was therefore active at every height between them. All targets are
proven from one checkpoint through ChainLocks, so they lie on one chain. A
target above `H` is never evidence that the quorum existed at `H`.

The 24 blocks, one DKG interval of the Platform quorum types, cover how Drive
rotates validator sets. It moves off its quorum only at the end of the first
block whose Core height drops that quorum from the active set
(`validator_set_update_v2` in
`packages/rs-drive-abci/src/execution/platform_events/block_end/validator_set_update/v2/mod.rs`),
so that block is still signed by a quorum Core has just rotated out at the
height it reports. The Core height can advance between consecutive Platform
blocks, and the rotation applies only after the block.

The checkpoint is trust configuration, like a hard-coded block hash: whoever
chooses it chooses the chain. Beyond it, the provider trusts that no ChainLock
quorum active at the checkpoint or mined after it ever signs dishonestly,
retired ones included, as `dash-core-proof` documents. A valid proof shows a
quorum was mined, not that its target is the newest ChainLock; how fresh a
response must be is the SDK's policy.

### Checkpoint shelf life

A proof covers every ChainLock quorum rotation between its starting state and
its target, so it grows as the checkpoint ages, and a relay has to keep the
blocks back to it. Each SDK release can move the embedded checkpoints forward.
A long-lived application should persist `latest_verified_state()` and pass it
back to `with_checkpoint` on the next start, so its proofs stay short.

### Limitations

- Keys are proven for the Core height Platform reports in its responses. If
  Platform halts, that height stops while Core moves on. A quorum not cached
  before the halt is still proven with a proof whose target is that height,
  as long as the relay's Core has that ChainLock.
- A height at or below the checkpoint cannot be proven, since every proof
  starts from it.
- If the Core height jumps by more than 24 blocks between two Platform blocks,
  for example on the first block after a long idle period or a halt, that one
  block's responses cannot be verified until the next Platform block. The
  provider reports the source as unavailable, so no node is banned.
- A node may name a retired quorum at an old height; it is proven from a
  state below that height, possibly the checkpoint. That is bounded by the
  same fetch limits and costs only cache churn, not safety.
- An inclusion proof cannot show that a quorum does not exist, so the
  provider never reports a quorum as absent; nodes naming fake quorums are
  skipped by the SDK, not banned.

### Refreshing the embedded checkpoints

`checkpoints/<network>.json` hold Core `ProofState` JSON (hashes in RPC display
order). To move one to a newer block `H`:

1. Pick a height with `chainlock: true` in `getblock` on two independent Core
   nodes, and confirm `getblockhash H` and the block hash on a block explorer
   agree.
2. Write the state from the block: `network` (0 mainnet, 1 testnet), `height`,
   `block_hash`, and the coinbase payload's `merkleRootMNList` and
   `merkleRootQuorums` as `masternode_root` and `quorum_root`.
3. Update `tests/data/checkpoint-evidence-<network>.json`: the raw header
   (`getblockheader <hash> false`), the raw coinbase transaction, and the
   coinbase's Merkle branch (sibling hashes in wire order). The
   `checkpoints` test recomputes the block hash, the header's Merkle root and
   both roots from this evidence.

## Implementation Details

Both providers implement the `ContextProvider` trait.

`VerifiedHttpContextProvider` provides:

1. **Quorum Public Keys**: proven from its checkpoint with DIP 175 proofs from
   its relays, and kept in an LRU cache
2. **Data Contracts**: pre-loaded or registered known contracts, then the
   system contracts enabled by cargo features, otherwise `None`
3. **Token Configurations**: pre-loaded known configurations, otherwise `None`
4. **Platform Activation Height**: hardcoded values for each network

`TrustedHttpContextProvider` provides:

1. **Quorum Public Keys**: Fetched from trusted HTTP endpoints with LRU caching
2. **Data Contracts**: 
   - First checks pre-loaded known contracts (if any)
   - Then delegates to the fallback provider if set
   - Otherwise returns `None`
3. **Token Configurations**: Delegated to the fallback provider if set, otherwise returns `None`
4. **Platform Activation Height**: Returns hardcoded values for each network

## License

MIT
