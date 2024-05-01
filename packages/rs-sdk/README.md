# Dash Platform Rust SDK

This is the official Rust SDK for the Dash Platform. Dash Platform is a Layer 2 cryptocurrency technology that builds upon the Dash layer 1 network. This SDK provides an abstraction layer to simplify usage of the Dash Platform along with data models based on the Dash Platform Protocol (DPP), a CRUD interface, and bindings for other technologies such as C.

See Rust documentation of this crate for more details.

## Quick start

### Cargo.toml

To use this crate, define it as a dependency in your `Cargo.toml`:

```toml
[dependencies]

dash-platform-sdk = { git="https://github.com/dashpay/platform"0 }
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
connection to the Platform.

You can see examples of mocking in [mock_fetch.rs](tests/fetch/mock_fetch.rs) and  [mock_fetch_many.rs](tests/fetch/mock_fetch_many.rs).

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
These vectors must be saved in `packages/rs-sdk/tests/vectors`.

### Generating test vectors

To generate test vectors for offline testing, you need to have access acredentials to Dash Platform instance, either by
specifying configuration manually in `packages/rs-sdk/tests/.env`. or starting a local devnet.

When you start local dev environment of Dash Platform using `yarn start`, the `.env` file is automatically generated during `yarn setup` or `yarn reset`, using `platform/scripts/configure_dotenv.sh` script. See [Dash Platform documentation](../../README.md) for more details about starting and using local devnet.

To generate test vectors:

1. Ensure platform address and credentials in `packages/rs-sdk/tests/.env` are correct.
2. Run  `packages/rs-sdk/scripts/generate_test_vectors.sh` script.
3. (Optional) commit generated files with `git commit packages/rs-sdk/tests/vectors/`.

### Running tests in offline mode

Run the offline test using the following command:

```bash
cargo test -p dash-platform-sdk
```

## Implementing Fetch and FetchAny on new objects

How to implement `Fetch` and `FetchAny` trait on new object types (`Object`).

It's basically copy-paste and tweaking of existing implementation for another object type.

Definitions:

1. `Request` - gRPC request type, as generated in `packages/dapi-grpc/protos/platform/v0/platform.proto`.
2. `Response` - gRPC response  type, as generated in `packages/dapi-grpc/protos/platform/v0/platform.proto`.
3. `Object` - object type that should be returned by rs-sdk, most likely defined in `dpp` crate.
   In some cases, it can be defined in `packages/rs-drive-proof-verifier/src/types.rs`.

Checklist:

1. [ ] Ensure protobuf messages are defined in `packages/dapi-grpc/protos/platform/v0/platform.proto` and generated
   correctly in `packages/dapi-grpc/src/platform/proto/org.dash.platform.dapi.v0.rs`.
2. [ ] In `packages/dapi-grpc/build.rs`, add `Request` to `VERSIONED_REQUESTS` and response `Response` to `VERSIONED_RESPONSES`.
   This should add derive of `VersionedGrpcMessage` (and some more) in `org.dash.platform.dapi.v0.rs`.
3. [ ] Link request and response type to dapi-client by adding appropriate invocation of `impl_transport_request_grpc!` macro
in `packages/rs-dapi-client/src/transport/grpc.rs`.
4. [ ] If needed, implement new type in `packages/rs-drive-proof-verifier/src/types.rs` to hide complexity of data structures
   used internally.

   If you intend to implement `FetchMany`, you should define type returned by `fetch_many()` using `RetrievedObjects`
   that will store collection of  returned objects, indexd by some key.
5. [ ] Implement `FromProof` trait for the `Object` (or type defined in `types.rs`) in `packages/rs-drive-proof-verifier/src/proof.rs`.
6. [ ] Implement `Query` trait for the `Request` in `packages/rs-sdk/src/platform/query.rs`.
7. [ ] Implement `Fetch\<Request\>` trait for the `Object` (or type defined in `types.rs`) in `packages/rs-sdk/src/platform/fetch.rs`.
8. [ ] Implement `FetchMany\<Request\>` trait for the `Object` (or type defined in `types.rs`) in `packages/rs-sdk/src/platform/fetch_many.rs`.
9. [ ] Add `mod ...;` clause to `packages/rs-sdk/tests/fetch/main.rs`
10. [ ] Implement unit tests in `packages/rs-sdk/tests/fetch/*object*.rs`
11. [ ] Add name of request type to match clause in `packages/rs-sdk/src/mock/sdk.rs` : `load_expectations()`
12. [ ] Start local devnet with `yarn reset ; yarn setup && yarn start`
13. [ ] Generate test vectors with script `packages/rs-sdk/scripts/generate_test_vectors.sh`
