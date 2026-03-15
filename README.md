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
  <a href="https://codecov.io/gh/dashpay/platform"><img alt="codecov" src="https://codecov.io/gh/dashpay/platform/branch/v3.1-dev/graph/badge.svg"></a>
  <a href="https://discordapp.com/invite/PXbUxJB"><img alt="General Chat" src="https://img.shields.io/badge/discord-General_chat-738adb"></a>
  <a href="https://twitter.com/intent/follow?screen_name=Dashpay"><img alt="Follow on Twitter" src="https://img.shields.io/twitter/follow/Dashpay.svg?style=social&label=Follow"></a>
</p>

<details>
<summary>Per-Crate Coverage</summary>

| Crate | Coverage |
|-------|----------|
| dpp | [![codecov](https://codecov.io/gh/dashpay/platform/branch/v3.1-dev/graph/badge.svg?component=dpp)](https://codecov.io/gh/dashpay/platform/component/dpp) |
| drive | [![codecov](https://codecov.io/gh/dashpay/platform/branch/v3.1-dev/graph/badge.svg?component=drive)](https://codecov.io/gh/dashpay/platform/component/drive) |
| drive-abci | [![codecov](https://codecov.io/gh/dashpay/platform/branch/v3.1-dev/graph/badge.svg?component=drive-abci)](https://codecov.io/gh/dashpay/platform/component/drive-abci) |
| sdk | [![codecov](https://codecov.io/gh/dashpay/platform/branch/v3.1-dev/graph/badge.svg?component=sdk)](https://codecov.io/gh/dashpay/platform/component/sdk) |

</details>

For in-depth architecture, internals, and developer documentation, see
[The Dash Platform Book](https://dashpay.github.io/platform/).

## What is Dash Platform

Dash Platform is a decentralized data storage and application layer built on top
of the Dash payment network. It lets developers store, query, and
cryptographically verify structured data on the Dash masternode network without
deploying or executing user-written code on-chain.

The central problem Dash Platform solves is: how do you let a light client --
a mobile wallet, a browser app, a third-party service -- query decentralized
state and **know** the answer is correct, without running a full node and
without trusting the node that served the response?

Platform's answer combines three pieces. First,
[Tenderdash](https://github.com/dashpay/tenderdash) runs Scalable Byzantine
Fault Tolerant (SBFT) consensus across a rotating quorum of masternodes. Unlike
classical BFT where every validator signs every block, Tenderdash selects a
deterministic quorum for each block and recovers a single BLS threshold
signature from that quorum. One compact signature attests to the entire
committed state. Second,
[GroveDB](https://github.com/dashpay/grovedb) stores all platform state in an
authenticated tree structure (a hierarchy of Merkle trees). Every piece of
data -- a document, an identity balance, a token supply -- has a Merkle path
from itself up to a single root hash. Third, **Drive** (this repository's core
component) ties these together: it commits the GroveDB root hash into
Tenderdash blocks. The threshold signature on a block therefore signs the state
root, and the state root cryptographically commits to every individual piece of
data in the system.

The result is that to verify any single query result, a client needs only three
things: the data itself, its Merkle proof against the state root, and the
threshold signature on that root. No full node, no chain of block headers, no
trust in the serving node. This is what makes Platform distinct from other
decentralized data systems: the combination of authenticated storage, BFT
consensus with threshold signatures, and proof-serving APIs gives light clients
the same security guarantees as full nodes.

### How it differs from smart contract platforms

Dash Platform is not a smart contract platform. There is no virtual machine, no
gas metering for code execution, and no user-deployed programs running on-chain.
Instead, developers define **data contracts** -- JSON Schema-based specifications
that describe the structure and validation rules for their application data. The
network stores, indexes, and enforces these schemas directly. Applications
interact with the platform through structured data reads and writes (called
**state transitions**) rather than arbitrary code execution. This eliminates
entire classes of smart contract vulnerabilities (reentrancy, unchecked external
calls, gas manipulation) and makes the system deterministic and predictable.

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

- **rs-drive** / **rs-drive-abci** -- Drive storage engine and ABCI application
- **rs-dpp** -- Dash Platform Protocol (data contracts, documents, state
  transitions, identities)
- **rs-sdk** -- Rust SDK for building applications on Dash Platform
- **wasm-sdk** / **wasm-dpp2** -- WebAssembly bindings for browser-based
  applications
- **rs-sdk-ffi** / **swift-sdk** -- FFI layer and iOS/Swift SDK
- **js-dash-sdk** / **js-evo-sdk** -- JavaScript SDKs
- **dashmate** -- Node management and local development tool
- **dapi** / **rs-dapi** -- Decentralized API server implementations

## Getting started

For installation, local development setup, and node operation, see the
[Getting Started](https://docs.dash.org/projects/platform/en/stable/docs/intro/what-is-dash-platform.html)
guide.

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
