# Dash Platform Rust SDK

This is the official Rust SDK for the Dash Platform. Dash Platform is a Layer 2 cryptocurrency technology that builds upon the Dash layer 1 network. This SDK provides an abstraction layer to simplify usage of the Dash Platform along with data models based on the Dash Platform Protocol (DPP), a CRUD interface, and bindings for other technologies such as C.

See Rust documentation of this crate for more details.

## Quick start

### Cargo.toml

To use this crate, define it as a dependency in your `Cargo.toml`:

```toml
[dependencies]

dash-sdk = { git="https://github.com/dashpay/platform" }
```

### Implementing Dash Platform SDK application

In order to build application that uses Dash Platform SDK, you need to:

1. Implement a [Wallet](src/wallet.rs) that will store, manage and use your keys to sign transactions and state transitions.
   An example implementation of wallet can be found in [src/mock/wallet.rs](src/mock/wallet.rs).
2. Implement Dash SPV client that will sync your application with Dash Core state, including quorum public keys.

   TODO: Add more details here.

   For testing and development purposes, while you don't have your SPV client implementation ready, you can setup local Dash Core node and access it using RPC interface (see below).

3. Implement  `ContextProvider` gives Dash Platform SDK access to state of your application, like:
   * quorum public keys retrieved using SPV,
   * data contracts configured and/or fetched from the server.

   See [GrpcContextProvider](../rs-sdk/src/mock/provider.rs) for an example implementation.

### Mocking

Dash Platform SDK supports mocking with `mocks` feature which provides a
convenient way to define mock expectations and use the SDK without actual
connection to Platform.

You can see examples of mocking in [mock_fetch.rs](tests/fetch/mock_fetch.rs) and  [mock_fetch_many.rs](tests/fetch/mock_fetch_many.rs).

## Transport-free consumption

The query-building, wire-encoding, and proof-verification layers of this SDK
live in the [`dash-platform-queries`](../dash-platform-queries) crate, which
this crate depends on and re-exports at the historical paths. Embedders that
bring their own transport and trust context (Dash Core's platform GUI, block
explorers) can depend on `dash-platform-queries` + `drive-proof-verifier`
directly and get typed, proof-verified results without `rs-dapi-client` or
tonic's native channel/TLS stack in their dependency tree. Shared generated
types and context-provider utilities remain dependencies. See that crate's
README for details.

## Examples

You can find quick start example in `examples/` folder. Examples must be configured by setting constants.

You can also inspect tests in `tests/` folder for more detailed examples.

Also refer to [Platform Explorer](https://github.com/dashpay/rs-platform-explorer/) which uses the SDK to execute various state transitions.

## Tests

This section provides instructions on how to test the RS-SDK for Dash Platform. The tests can be run in two modes: **offline** (without connectivity to the Dash Platform) and **network** (with connectivity to the Dash Platform). **Offline** mode is the default one.

If both **network** and **offline** testing is enabled, **offline testing** takes precedence.

## Network Testing

Network testing requires connectivity to the Dash Platform and Dash Core.

Follow these steps to conduct network testing:

1. Configure platform address and credentials in `packages/rs-sdk/tests/.env`.
   Note that the `.env` file might already be configured during  project setup (`yarn setup`).
2. Run the test without default features, but with `network-testing` feature enabled.

```bash
cd packages/rs-sdk
cargo test -p dash-sdk --no-default-features --features network-testing
```

## Offline Testing

Offline testing uses the vectors generated using `packages/rs-sdk/scripts/generate_test_vectors.sh` script.
This script will connect to node defined in `packages/rs-sdk/tests/.env`, execute all tests against it and
update test vectors in `packages/rs-sdk/tests/vectors`.

To generate test vectors against a testnet node (or other remote node), you can use helper script
`packages/rs-sdk/scripts/connect_to_remote.sh` which will generate `.env` file for you and tunnel connection to Dash
Core RPC on the remote host.

Refer to rich comments / help in the forementioned scripts for more details.

### SDK test data

When starting the local devnet with `yarn start` (the `local` dashmate config has `buildArgs.SDK_TEST_DATA = "true"` set by `yarn setup` — see `scripts/configure_dashmate.sh`), the `create_sdk_test_data` cfg flag
activates creation of deterministic test data in genesis state. This data is defined in
`packages/rs-drive-abci/src/execution/platform_events/initialization/create_genesis_state/test/`.

Current test data includes 3 identities, a data contract with 3 tokens, and address balances.
Token configuration:

| Token | Config |
|-------|--------|
| `TOKEN_ID_0` | base_supply=100000, frozen for IDENTITY_ID_2, no pricing, no pre-programmed distributions |
| `TOKEN_ID_1` | base_supply=100000, paused, single price=25, no pre-programmed distributions |
| `TOKEN_ID_2` | base_supply=100000, pricing schedule (10 levels), pre-programmed distributions at timestamps 1000, 5000, 10000 |

When adding a new query type, add corresponding test data to the files in `create_genesis_state/test/`
and reference it in `packages/rs-sdk/tests/fetch/generated_data.rs`.

### Generating test vectors

To generate test vectors for offline testing, you need to have access to a Dash Platform instance, either by
specifying configuration manually in `packages/rs-sdk/tests/.env` or starting a local devnet.
The `.env` file is automatically generated during `yarn setup` or `yarn reset`, using `platform/scripts/configure_dotenv.sh` script. See [Dash Platform documentation](../../README.md) for more details about starting and using local devnet.

To generate test vectors:

1. Start local dev environment of Dash Platform using `yarn start` (the `local` dashmate config has `buildArgs.SDK_TEST_DATA = "true"` set by `yarn setup` — see `scripts/configure_dashmate.sh`).
2. Ensure platform address and credentials in `packages/rs-sdk/tests/.env` are correct.
3. Run  `packages/rs-sdk/scripts/generate_test_vectors.sh` script.
4. (Optional) commit generated files with `git commit packages/rs-sdk/tests/vectors/`.

### Running tests in offline mode

Run the offline test using the following command:

```bash
cargo test -p dash-platform-sdk
```

## Implementing Fetch and FetchMany on new objects

How to implement `Fetch` and `FetchMany` trait on new object types (`Object`).

It's basically copy-paste and tweaking of existing implementation for another object type.

Definitions:

1. `Request` - gRPC request type, as generated in `packages/dapi-grpc/protos/platform/v0/platform.proto`.
2. `Response` - gRPC response  type, as generated in `packages/dapi-grpc/protos/platform/v0/platform.proto`.
3. `Object` - object type that should be returned by rs-sdk, most likely defined in `dpp` crate.
   In some cases, it can be defined in `packages/rs-drive-proof-verifier/src/types.rs`.
4. `Key` - some unique identifier of the `Object`, for example `platform_value::Identifier`

Checklist:

1. [ ] Ensure protobuf messages are defined in `packages/dapi-grpc/protos/platform/v0/platform.proto` and generated
   correctly in `packages/dapi-grpc/src/platform/client/org.dash.platform.dapi.v0.rs`.
2. [ ] In `packages/dapi-grpc/build.rs`, add `Request` to `VERSIONED_REQUESTS` and response `Response` to `VERSIONED_RESPONSES`.
   This should add derive of `VersionedGrpcMessage` (and some more) in `org.dash.platform.dapi.v0.rs`.
3. [ ] Link request and response type to dapi-client by adding appropriate invocation of `impl_transport_request_grpc!` macro
in `packages/rs-dapi-client/src/transport/grpc.rs`.
4. [ ] If needed, implement new type in `packages/rs-drive-proof-verifier/src/types.rs` to hide complexity of data structures
   used internally.

   If you intend to implement `FetchMany`, you should define type returned by `fetch_many()` using `RetrievedObjects`
   that will store collection of  returned objects, indexed by some key.
5. [ ] Implement `FromProof` trait for the `Object` (or type defined in `types.rs`) in `packages/rs-drive-proof-verifier/src/proof.rs`.
6. [ ] Implement `Query` trait for the `Request` and `Fetch` (or `FetchMany`) trait for the `Object`.
   Create a dedicated module under the appropriate subdirectory
   (e.g., `packages/rs-sdk/src/platform/tokens/my_query.rs`) and add `Query` + `Fetch`/`FetchMany` impls there.
   **Deprecated:** older code placed these in central files (`query.rs`, `fetch.rs`, `fetch_many.rs`) — do not follow that pattern for new queries.
7. [ ] Implement `MockResponse` for `Object` in `packages/rs-sdk/src/mock/requests.rs`.
8. [ ] Implement `FetchMany\<Key\>` trait for the `Object` (or type defined in `types.rs`),
   with inner type Request = `Request`, if the query returns a collection of objects.
   Skip if the query returns a single result and only `Fetch` is needed.
9. [ ] Add `mod ...;` clause to `packages/rs-sdk/tests/fetch/main.rs`
10. [ ] Implement unit tests in `packages/rs-sdk/tests/fetch/*object*.rs`.
    Tests must compile but will **fail** at this point due to missing test vectors — that is expected.
    The vector generation script (step 13) runs these tests with `--features generate-test-vectors`
    against a live devnet to record responses as test vectors. After vectors are generated,
    re-run `cargo test -p dash-sdk` to verify they pass in offline mode.
11. [ ] Add name of request type to match clause in `packages/rs-sdk/src/mock/sdk.rs` : `load_expectations()`
12. [ ] (Optional) If not already configured, run `yarn setup` (fresh checkout) or `yarn reset` (reconfigure existing environment).
    **Warning:** both commands rebuild everything and reset data — do not run if your environment is already working.
    This configures the `.env` file in `packages/rs-sdk/tests/` needed by the tests.
13. [ ] Start local devnet with `yarn start` (the `local` dashmate config has `buildArgs.SDK_TEST_DATA = "true"` set by `yarn setup` — see `scripts/configure_dashmate.sh`).
14. [ ] Generate test vectors by running `packages/rs-sdk/scripts/generate_test_vectors.sh test_name`
    where `test_name` matches only the new tests (e.g., `test_token_pre_programmed_distributions`).
    Running without arguments regenerates **all** vectors — avoid this unless intentional.
    The script executes matching tests with `--features generate-test-vectors` against the running devnet,
    saving responses as test vectors in `packages/rs-sdk/tests/vectors/`.
