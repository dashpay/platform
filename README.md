<!-- markdownlint-disable MD033 MD041 -->
<p align="center">
  <a href="https://dashplatform.readme.io/docs/introduction-what-is-dash-platform/">
    <img alt="Dash" src="https://media.dash.org/wp-content/uploads/dash_digital-cash_logo_2018_rgb_for_screens.png" width="546">
  </a>
</p>

<p align="center">
  Seriously fast decentralized applications for the Dash network
</p>

<p align="center">
  <a href="https://github.com/dashpay/platform/actions/workflows/tests.yml"><img alt="GitHub CI Status" src="https://github.com/dashpay/platform/actions/workflows/tests.yml/badge.svg"></a>
  <a href="NIGHTLY_STATUS.md"><img alt="Nightly Tests" src="https://github.com/dashpay/platform/actions/workflows/tests.yml/badge.svg?event=schedule" title="Nightly test status"></a>
  <a href="https://codecov.io/gh/dashpay/platform"><img alt="codecov" src="https://codecov.io/gh/dashpay/platform/branch/v3.1-dev/graph/badge.svg"></a>
  <a href="https://github.com/dashpay/platform/graphs/commit-activity"><img alt="commit activity" src="https://img.shields.io/github/commit-activity/m/dashpay/platform"></a>
  <a href="https://github.com/dashpay/platform/commits"><img alt="last commit" src="https://img.shields.io/github/last-commit/dashpay/platform"></a>
  <a href="https://discordapp.com/invite/PXbUxJB"><img alt="General Chat" src="https://img.shields.io/badge/discord-General_chat-738adb"></a>
  <a href="https://twitter.com/intent/follow?screen_name=Dashpay"><img alt="Follow on Twitter" src="https://img.shields.io/twitter/follow/Dashpay.svg?style=social&label=Follow"></a>
</p>

<details>
<summary>Per-Crate Coverage</summary>

| Crate | Lines | Coverage |
|-------|------:|----------|
| [rs-dpp](./packages/rs-dpp) | 129k | [![codecov](https://codecov.io/gh/dashpay/platform/branch/v3.1-dev/graph/badge.svg?component=dpp)](https://codecov.io/gh/dashpay/platform/component/dpp) |
| [rs-drive](./packages/rs-drive) | 171k | [![codecov](https://codecov.io/gh/dashpay/platform/branch/v3.1-dev/graph/badge.svg?component=drive)](https://codecov.io/gh/dashpay/platform/component/drive) |
| [rs-drive-abci](./packages/rs-drive-abci) | 125k | [![codecov](https://codecov.io/gh/dashpay/platform/branch/v3.1-dev/graph/badge.svg?component=drive-abci)](https://codecov.io/gh/dashpay/platform/component/drive-abci) |
| [rs-sdk](./packages/rs-sdk) | 23k | [![codecov](https://codecov.io/gh/dashpay/platform/branch/v3.1-dev/graph/badge.svg?component=sdk)](https://codecov.io/gh/dashpay/platform/component/sdk) |

</details>

For in-depth architecture, internals, and developer documentation, see
[The Dash Platform Book](https://dashpay.github.io/platform/).

## What is Dash Platform

Dash Platform is a decentralized data storage and application layer built on top
of the Dash payment network. It lets developers store, query, and
cryptographically verify structured data on the Dash masternode network without
deploying or executing user-written code on-chain.

Instead of smart contracts, developers define **data contracts** -- JSON
Schema-based specifications that describe the structure and validation rules for
their application data. The network stores, indexes, and enforces these schemas
directly. Applications interact with the platform through structured data reads
and writes (called **state transitions**) rather than arbitrary code execution.
Smart contract support is planned for Platform v4.0 (targeted for mainnet in
2027).

### How Dash Platform compares

| | Ethereum | Solana | Dash Platform |
|---|---|---|---|
| **Primary purpose** | General-purpose smart contracts | High-throughput smart contracts | Decentralized data storage and querying |
| **Consensus** | Gasper (PoS) | Tower BFT (PoS) | Tenderdash SBFT (masternode quorums, BLS threshold signatures) |
| **Finality** | ~13 min (2 epochs) | **~0.4s (optimistic)** | **Instant (1 block)** |
| **Decentralized querying** | Keys only (no native indexing) | Keys only (via RPC, no proofs) | **Rich queries with indexes, ordering, and ranges -- all with proofs** |
| **State proofs** | Merkle-Patricia proofs | No native proofs | **GroveDB Merkle proofs for every query** |
| **Light client trust** | Needs sync committee | Trusts RPC provider | **Cryptographic proof per response -- same security as a full node** |
| **Data model** | Account / key-value | Account / key-value | **Structured documents with secondary indexes** |
| **Smart contracts** | **Yes (Solidity / Vyper on EVM)** | **Yes (Rust / C on SVM)** | Coming in v4.0 |

The standout difference is light client verification. Most chains either offer
no state proofs (Solana) or give proofs that are expensive to verify
(Ethereum's sync committee). Dash Platform serves a cryptographic proof with
every query response, and a single BLS threshold signature is all a client needs
to verify it. A mobile wallet gets the same security guarantees as a full node.

For a comprehensive comparison across more chains, see the
[Platform Comparison](https://dashpay.github.io/platform/platform-comparison.html)
chapter in The Dash Platform Book.

### Architecture deep dive

The central problem Dash Platform solves is: how do you let a light client --
a mobile wallet, a browser app, a third-party service -- query decentralized
state and **know** the answer is correct, without running a full node and
without trusting the node that served the response?

Platform's answer stacks four layers. At the bottom,
[GroveDB](https://github.com/dashpay/grovedb) provides the raw authenticated
storage -- a hierarchy of Merkle trees where every key-value pair has a
cryptographic path up to a single root hash. GroveDB handles insertions,
deletions, and proof generation, but it knows nothing about documents,
identities, or application logic. Think of it as Platform's assembly language:
powerful and provable, but operating at the level of individual tree operations.

**Drive** is the layer that gives GroveDB meaning -- much as C gives structure
and abstraction over assembly. Drive organizes GroveDB's raw trees into a
structured query system with secondary indexes: it defines how documents,
identities, balances, tokens, and data contracts are laid out across the tree
hierarchy, how indexes are maintained, and how queries are translated into
authenticated tree operations. When an application asks "give me all documents
where `owner = X`, sorted by `createdAt`", it is Drive that maps that query onto
the right set of GroveDB tree traversals and returns a result with a proof.

Above Drive sits **Drive-ABCI**, the execution layer that ties everything
together. It connects Drive to
[Tenderdash](https://github.com/dashpay/tenderdash) consensus via ABCI,
validates and applies state transitions, enforces protocol rules, and commits
the GroveDB root hash into each consensus block. Tenderdash itself runs Scalable
Byzantine Fault Tolerant (SBFT) consensus across a rotating quorum of
masternodes. Unlike classical BFT where every validator signs every block,
Tenderdash selects a deterministic quorum for each block and recovers a single
BLS threshold signature from that quorum. One compact signature attests to the
entire committed state, and that state root cryptographically commits to every
individual piece of data in the system.

The result is that to verify any single query result, a client needs only three
things: the data itself, its Merkle proof against the state root, and the
threshold signature on that root. No full node, no chain of block headers, no
trust in the serving node.

### Key capabilities

**Identities and naming.** Users register identities on-chain -- first-class
protocol objects with hierarchical key management, not just addresses. Identities
can hold multiple authentication and encryption keys with different security
levels and purposes. The Dash Platform Naming Service (DPNS) maps human-readable
usernames to identities, resolved directly by the network.

**Credits and fees.** Users convert Dash into credits that pay for storage and
state transitions. Fees are deterministic and based on the actual storage and
processing cost of each operation. The platform supports transparent fee payment
from Dash addresses as well as private fee payment through a shielded pool using
Orchard-based zero-knowledge proofs (Halo2).

**Tokens.** The platform supports user-created tokens with protocol-enforced
rules for minting, burning, transferring, and freezing. Token behavior is
configured declaratively through data contract definitions. Pre-programmed
distributions, group-based minting authority, and manual/managed supply models
are all native protocol features rather than user-written contract logic.

**Decentralized API.** Clients interact with the network through DAPI, a gRPC
interface served by every masternode. There is no central API server, gateway, or
RPC provider. Any masternode can serve any request, and every response can carry
a proof. DAPI provides endpoints for querying documents, broadcasting state
transitions, and verifying proofs.

For a detailed treatment of each of these areas, see
[The Dash Platform Book](https://dashpay.github.io/platform/).

## Foundation libraries

Dash Platform builds on several standalone libraries developed by the Dash
project:

- [Tenderdash](https://github.com/dashpay/tenderdash) -- SBFT consensus engine
  (Go), a fork of Tendermint redesigned for Dash's masternode quorum model
- [rs-tenderdash-abci](https://github.com/dashpay/rs-tenderdash-abci) -- Rust
  ABCI interface for connecting Drive to Tenderdash
- [GroveDB](https://github.com/dashpay/grovedb) -- authenticated key-value
  store built on hierarchical Merkle trees, providing proof generation for
  arbitrary queries
- [rust-dashcore](https://github.com/dashpay/rust-dashcore) -- Rust
  implementation of Dash Core primitives (transactions, blocks, BLS keys,
  addresses)

## Repository structure

This is a monorepo containing all packages that comprise Dash Platform. Packages
are located in the [packages](./packages) directory. Key packages include:

- **rs-drive** -- Drive query and indexing layer over GroveDB
- **rs-drive-abci** -- Execution layer connecting Drive to Tenderdash consensus
- **rs-dpp** -- Dash Platform Protocol (data contracts, documents, state
  transitions, identities)
- **rs-sdk** -- Rust SDK for building applications on Dash Platform
- **wasm-sdk** / **wasm-dpp2** -- WebAssembly bindings for browser-based
  applications
- **rs-sdk-ffi** / **swift-sdk** -- FFI layer and iOS/Swift SDK
- **js-evo-sdk** -- JavaScript SDK
- **dashmate** -- Node management and local development tool
- **dapi** / **rs-dapi** -- Decentralized API server implementations

## SDK support

| SDK | Status | Package |
|-----|--------|---------|
| **Rust** | Available now | [`rs-sdk`](./packages/rs-sdk) |
| **JavaScript** | Available now | [`js-evo-sdk`](./packages/js-evo-sdk) |
| **iOS (Swift)** | Coming in v3.1 | [`swift-sdk`](./packages/swift-sdk) |
| **Android** | Coming in v3.2 | -- |

For details on choosing an SDK and what each one provides, see the
[SDK Support](https://dashpay.github.io/platform/sdk-support.html) chapter in
The Dash Platform Book.

## Getting started

For prerequisites, local development setup, and build instructions, see the
[Getting Started](https://dashpay.github.io/platform/getting-started.html)
chapter in The Dash Platform Book.

## Contributing

- Join the [Dash Discord](https://discordapp.com/invite/PXbUxJB) for questions
  and discussion
- Read [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on submitting issues
  and pull requests
- See [AGENTS.md](AGENTS.md) for a concise contributor guide covering repo
  structure, commands, style, and tests
- File issues and feature requests at
  [platform/issues](https://github.com/dashpay/platform/issues)

## License

[MIT](LICENSE.md)
