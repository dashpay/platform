# Platform Comparison

How Dash Platform compares to other blockchain networks across architecture,
features, and developer experience. Ratings from `-` (not supported) through
`+` (basic), `++` (good), to `+++` (best in class) reflect relative strength
in each dimension.

## Overview

| | Bitcoin | Ethereum | Solana | Polkadot | NEAR | Cosmos SDK | Avalanche | Dash Platform |
|---|---|---|---|---|---|---|---|---|
| **Primary purpose** | Payments | General-purpose smart contracts | High-throughput smart contracts | Multi-chain shared security | Sharded smart contracts | App-chain framework | Multi-chain smart contracts | Decentralized data storage and querying |
| **Consensus** | Nakamoto (PoW) | Gasper (PoS) | Tower BFT (PoS) | GRANDPA + BABE (PoS) | Nightshade (PoS) | CometBFT (PoS) | Snowman (PoS) | Tenderdash SBFT (masternode quorums, BLS threshold signatures) |
| **Finality** | `-` Probabilistic (~60 min) | `+` ~13 min (2 epochs) | `+++` ~0.4s (optimistic) | `+` ~12-60s (2 rounds) | `++` ~1-2s | `+++` Instant (1 block) | `++` ~1-2s | `+++` Instant (1 block) |
| **Throughput** | `-` ~7 tx/s | `+` ~15-30 tx/s | `+++` ~65,000 tx/s | `++` Scales per parachain | `++` ~100,000 tx/s (sharded) | `++` Per-chain | `++` ~4,500 tx/s | `+` ~200 tx/s |

## Data and Querying

| | Bitcoin | Ethereum | Solana | Polkadot | NEAR | Cosmos SDK | Avalanche | Dash Platform |
|---|---|---|---|---|---|---|---|---|
| **Data model** | `-` UTXOs | `+` Account / key-value | `+` Account / key-value | `+` Account / key-value | `+` Account / key-value | `+` App-defined | `+` Account / key-value | `+++` Structured documents with secondary indexes |
| **Decentralized querying** | `-` Keys only (UTXO lookup) | `+` Keys only (no native indexing) | `+` Keys only (via RPC, no proofs) | `+` Keys only (per parachain) | `+` Keys only (via RPC, no proofs) | `+` Keys only (app-specific) | `+` Keys only (via RPC, no proofs) | `+++` Rich queries with indexes, ordering, and ranges -- all with proofs |
| **State proofs** | `+` SPV (block headers) | `++` Merkle-Patricia proofs | `-` No native proofs | `+` Merkle proofs (per parachain) | `++` Merkle-Patricia proofs | `+` IAVL proofs | `+` Merkle proofs | `+++` GroveDB Merkle proofs for every query |
| **Light client trust** | `+` Follows longest chain | `+` Needs sync committee | `-` Trusts RPC provider | `+` Trusts relay chain | `-` Trusts RPC provider | `+` Trusts IBC relayer | `-` Trusts RPC provider | `+++` Cryptographic proof per response -- same security as a full node |

The standout difference is light client verification. Most chains either offer
no state proofs (Solana, Avalanche), require trusting intermediaries (Polkadot's
relay chain, Cosmos IBC relayers, NEAR's RPC providers), or give proofs that are
expensive to verify (Ethereum's sync committee). Dash Platform serves a
cryptographic proof with every query response, and a single BLS threshold
signature is all a client needs to verify it. A mobile wallet gets the same
security guarantees as a full node.

## Smart Contracts and Programmability

| | Bitcoin | Ethereum | Solana | Polkadot | NEAR | Cosmos SDK | Avalanche | Dash Platform |
|---|---|---|---|---|---|---|---|---|
| **Smart contracts** | `-` Limited Script opcodes | `+++` Solidity / Vyper on EVM | `+++` Rust / C on SVM | `++` Per-parachain, typically Wasm | `++` Rust / JS / AssemblyScript on Wasm VM | `+` App-specific (Go) | `++` Solidity on EVM, Rust on Wasm | `-` Coming in v4.0 |
| **VM / execution** | `-` Script interpreter | `+++` EVM | `+++` SVM (eBPF) | `++` Wasm (per parachain) | `++` Wasm VM | `+` No VM (compiled Go) | `++` EVM + Wasm subnets | `-` No VM (data contracts; VM planned for v4.0) |
| **Developer languages** | `-` Script | `+++` Solidity, Vyper | `++` Rust, C | `++` Rust (Substrate) | `++` Rust, JS, AssemblyScript | `+` Go | `++` Solidity, Rust | `+` JSON Schema (data contracts), Rust/JS/Swift (SDKs) |
| **Smart contract security** | `+++` No VM attack surface | `+` Reentrancy, gas exploits | `++` No reentrancy, but complexity | `++` Sandboxed per parachain | `++` Wasm sandboxing | `++` No VM attack surface | `+` Inherits EVM risks | `+++` No VM attack surface (data contracts are declarative) |

Dash Platform takes a fundamentally different approach: instead of a VM that
executes arbitrary code, developers define **data contracts** -- JSON
Schema-based specifications that describe the structure and validation rules for
their application data. The network stores, indexes, and enforces these schemas
directly. This eliminates entire classes of smart contract vulnerabilities
(reentrancy, unchecked external calls, gas manipulation). Smart contract support
is planned for Platform v4.0 (targeted for mainnet in 2027).

## Token Support

| | Bitcoin | Ethereum | Solana | Polkadot | NEAR | Cosmos SDK | Avalanche | Dash Platform |
|---|---|---|---|---|---|---|---|---|
| **Native token standard** | `-` BRC-20 via inscriptions | `+++` ERC-20 / ERC-721 / ERC-1155 | `++` SPL Token | `+` Per-parachain | `++` NEP-141 / NEP-171 | `+` Per-chain | `++` ERC-20 (C-Chain) | `+++` Protocol-native tokens with declarative rules |
| **Token creation** | `-` Requires third-party indexer | `++` Deploy smart contract | `++` Deploy program | `+` Deploy parachain | `++` Deploy smart contract | `+` Build app-chain | `++` Deploy smart contract | `+++` Declare in data contract -- no code deployment |
| **Freeze / pause** | `-` No | `+` Only if contract implements it | `++` Mint authority can freeze | `+` Per-parachain | `+` Only if contract implements it | `+` Per-chain | `+` Only if contract implements it | `+++` Protocol-level freeze, pause, and destroy |
| **Minting authority** | `-` N/A | `+` Contract owner / governance | `+` Mint authority | `+` Per-parachain | `+` Contract owner | `+` Per-chain | `+` Contract owner | `+++` Individuals, groups with threshold signing, or pre-programmed schedules |
| **Pre-programmed distributions** | `-` No | `+` Requires contract logic | `+` Requires program logic | `-` No | `+` Requires contract logic | `+` Requires app logic | `+` Requires contract logic | `+++` Native: time-based, epoch-based perpetual distributions |

Dash Platform tokens are first-class protocol objects rather than smart contract
deployments. Token behavior (minting rules, supply caps, freeze authority,
distribution schedules) is configured declaratively in data contracts and
enforced by the protocol itself.

## Project and Ecosystem

| | Bitcoin | Ethereum | Solana | Polkadot | NEAR | Cosmos SDK | Avalanche | Dash Platform |
|---|---|---|---|---|---|---|---|---|
| **License** | MIT | Various (GPL, Apache, MIT) | Apache 2.0 | GPL 3.0 | Apache 2.0 / MIT | Apache 2.0 | BSD 3-Clause | MIT |
| **Open source** | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Yes |
| **Core language** | C++ | Go, Rust | Rust | Rust | Rust | Go | Go | Rust |
| **Client SDKs** | `+` Multiple (community) | `+++` web3.js, ethers.js, viem | `++` @solana/web3.js | `+` Polkadot.js | `+` near-api-js | `+` CosmJS | `++` ethers.js (C-Chain) | `++` Rust, JavaScript, Swift (iOS), Android (coming) |
| **Launched** | 2009 | 2015 | 2020 | 2020 | 2020 | 2019 (SDK) | 2020 | 2024 (v1.0 mainnet) |
| **Ecosystem maturity** | `+++` Largest, most established | `+++` Largest smart contract ecosystem | `++` Fast-growing DeFi ecosystem | `+` Growing parachain ecosystem | `+` Growing dApp ecosystem | `++` Many sovereign chains | `++` Growing subnet ecosystem | `+` Early stage, growing |
| **Identity system** | `-` Addresses only | `+` ENS (contract-based) | `-` No native identity | `-` No native identity | `+` Named accounts | `-` No native identity | `-` No native identity | `+++` Protocol-native identities with hierarchical keys and DPNS usernames |
| **Native token privacy** | `+` UTXO model allows address rotation | `-` Account model, all activity linked to one address | `-` Account model, fully transparent | `-` Per-parachain, generally transparent | `-` Account model, fully transparent | `-` Generally transparent | `-` Account model, fully transparent | `+++` Shielded pool with Orchard/Halo2 ZK proofs |
