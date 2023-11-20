# Dash Platform Rust SDK

This is the official Rust SDK for the Dash Platform. Dash Platform is a Layer 2 cryptocurrency technology that builds upon the Dash layer 1 network. This SDK provides an abstraction layer to simplify usage of the Dash Platform along with data models based on the Dash Platform Protocol (DPP), a CRUD interface, and bindings for other technologies such as C.

See Rust documentation of this crate for more details.

## Examples

You can find quick start example in `examples/` folder. Examples must be configured by setting constants.

You can also inspect tests in `tests/` folder for more detailed examples.

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
cargo test -p rs-sdk --no-default-features --features network-testing
```

## Offline Testing

Offline testing uses the vectors generated using `packages/rs-sdk/scripts/generate_test_vectors.sh` script.
These vectors must be saved in `packages/rs-sdk/tests/vectors`.

### Generating test vectors

To generate test vectors for offline testing:

1. Configure platform address and credentials in `packages/rs-sdk/tests/.env`.
   Note that the `.env` file might already be configured during project setup (`yarn setup`).
2. Run  `packages/rs-sdk/scripts/generate_test_vectors.sh` script.

### Running tests in offline mode

Run the offline test using the following command:

```bash
cargo test -p rs-sdk
```
