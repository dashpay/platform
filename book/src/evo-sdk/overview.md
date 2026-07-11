# Evo SDK Overview

The **Evo SDK** (`@dashevo/evo-sdk`) is the primary JavaScript/TypeScript SDK
for building applications on Dash Platform. It provides a high-level,
strongly-typed facade over the WebAssembly-based Rust SDK, working in both
Node.js (≥ 18.18) and modern browsers.

> **API reference**: For detailed per-method documentation with interactive
> examples, see the [Evo SDK Docs](https://dashpay.github.io/evo-sdk-website/docs.html).

## How it works

```text
┌──────────────────┐
│  Your TypeScript  │
│   Application     │
└────────┬─────────┘
         │  EvoSDK facades (identities, documents, tokens, …)
┌────────▼─────────┐
│   @dashevo/       │
│   evo-sdk         │  TypeScript wrapper layer
└────────┬─────────┘
         │  calls into compiled WASM module
┌────────▼─────────┐
│   @dashevo/       │
│   wasm-sdk        │  Rust SDK compiled to WebAssembly
└────────┬─────────┘
         │  gRPC over HTTPS
┌────────▼─────────┐
│  DAPI nodes       │  Dash Platform's decentralized API
└──────────────────┘
```

The Evo SDK does **not** use JSON-RPC or REST. Every request is a gRPC call to
one of the Platform's DAPI nodes. Responses include cryptographic proofs that
the SDK verifies against the platform state root, so you do not need to trust
any single node.

## Facades

The SDK organises its API into domain-specific facades, each accessible as a
property on the `EvoSDK` instance:

| Facade | Description |
|--------|-------------|
| `sdk.identities` | Fetch, create, update, and top up identities |
| `sdk.contracts` | Fetch, publish, and update data contracts |
| `sdk.documents` | Query, create, replace, delete, and transfer documents |
| `sdk.tokens` | Mint, burn, transfer, freeze tokens and query balances |
| `sdk.dpns` | Register and resolve Dash Platform names |
| `sdk.addresses` | Query balances, transfer credits, withdraw to L1 |
| `sdk.epoch` | Query epoch information and evonode proposed blocks |
| `sdk.protocol` | Protocol version upgrade state and voting |
| `sdk.stateTransitions` | Broadcast and wait for state transitions |
| `sdk.system` | System status, quorum info, and total credits |
| `sdk.group` | Group membership, actions, and contested resources |
| `sdk.voting` | Contested resource vote states and polls |

A standalone `wallet` namespace is also exported for mnemonic generation, key
derivation, address validation, and message signing — see the
[Wallet Utilities](wallet-utilities.md) chapter.

## What it covers

The SDK supports the full set of Platform operations:

- **Queries** (read-only): fetch identities, contracts, documents, token
  balances, DPNS names, epoch info, vote states, and more. Every query can
  return a cryptographic proof.
- **State transitions** (writes): create identities, deploy contracts, manage
  documents and tokens, register names, cast votes, and transfer credits.

See the [API reference](https://dashpay.github.io/evo-sdk-website/docs.html) for the
complete list of operations with interactive examples.
