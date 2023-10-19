# Dash Platform Rust SDK

This is the official Rust SDK for the Dash Platform. Dash Platform is a Layer 2 cryptocurrency technology that builds upon the Dash layer 1 network. This SDK provides an abstraction layer to simplify usage of the Dash Platform along with data models based on the Dash Platform Protocol (DPP), a CRUD interface, and bindings for other technologies such as C.

See Rust documentation of this crate for more details.

## Examples

You can find quick start examples in `examples/` folder.

## Tests

This section provides instructions on how to test the RS-SDK for Dash Platform. The tests can be run in two modes: **offline** (without connectivity to the Dash Platform) and **online** (with connectivity to the Dash Platform). **Offline** mode is the default one.

## Online Testing

Online testing requires connectivity to the Dash Platform and Dash Core. This mode generates new test vectors that can be used in offline mode.

Follow these steps to conduct online testing:

1. Configure the environment variables in `packages/rs-sdk/.env`. Refer to the "Test Configuration" section below.
2. Optionally, you can remove existing test vectors.
3. Run the test with the `online-testing` feature.

Use the following commands for the above steps:

```bash
cd packages/rs-sdk
rm tests/vectors/*
cargo test -p rs-sdk --features online-testing
```

## Offline Testing

Offline testing uses the vectors generated in online mode. These vectors must be saved in `packages/rs-sdk/tests/vectors`.

Run the offline test using the following command:

```bash
cargo test -p rs-sdk
```

## Test Configuration

For the `online-testing` feature, you need to set the configuration in the environment variables or in `packages/rs-sdk/.env` file. You can refer to `packages/rs-sdk/.env.example` for the format.

The identifiers are generated with the platform test suite. To display them, apply the following diff:

```diff
diff --git a/packages/platform-test-suite/test/functional/platform/Document.spec.js b/packages/platform-test-suite/test/functional/platform/Document.spec.js
index 29dca311b..fba0aefc2 100644
--- a/packages/platform-test-suite/test/functional/platform/Document.spec.js
+++ b/packages/platform-test-suite/test/functional/platform/Document.spec.js
@@ -180,6 +180,9 @@ describe('Platform', () => {
 
       // Additional wait time to mitigate testnet latency
       await waitForSTPropagated();
+      console.log("Owner ID: " + document.getOwnerId().toString("base58"));
+      console.log("Data Contract: " + document.getDataContractId().toString("base58"));
+      console.log("Document: " + document.getId().toString("base58"));
     });
 
     it('should fetch created document', async () => {

```

To run the document test, use the following commands:

```bash
cd packages/platform-test-suite/
yarn mocha -b test/functional/platform/Document.spec.js
```

Find the values in the output and copy them to `packages/rs-sdk/.env`.
