# Nightly Test Status

[![Nightly Tests](https://github.com/dashpay/platform/actions/workflows/tests.yml/badge.svg?event=schedule)](https://github.com/dashpay/platform/actions/workflows/tests.yml?query=event%3Aschedule)

Nightly tests run every day at **23:00 UTC** on the `v3.1-dev` branch via the [Tests workflow](https://github.com/dashpay/platform/actions/workflows/tests.yml?query=event%3Aschedule). They exercise the full CI pipeline including Docker image builds, E2E tests, and the platform test suite.

## Current Status (as of 2026-03-21)

### Passing

| Job | Status | Notes |
|-----|--------|-------|
| [Build JS packages](https://github.com/dashpay/platform/actions/workflows/tests-build-js.yml) | Passing | |
| [Build Docker images](https://github.com/dashpay/platform/actions/workflows/tests-build-image.yml) (Drive, RS-DAPI, Dashmate helper) | Passing | All 3 images build successfully |
| [Rust workspace tests](https://github.com/dashpay/platform/actions/workflows/tests-rs-workspace.yml) (macOS) | Passing | |
| [Swift SDK build](https://github.com/dashpay/platform/actions/workflows/swift-sdk-build.yml) | Passing | |
| [JS package tests](https://github.com/dashpay/platform/actions/workflows/tests-js-package.yml) | Passing | dapi, dapi-client, evo-sdk, wallet-lib, wasm-dpp, wasm-dpp2, wasm-sdk, dash, dashmate |
| [JS dependency versions](https://github.com/dashpay/platform/actions/workflows/tests.yml) | Passing | `yarn constraints` check |
| [Dashmate E2E tests](https://github.com/dashpay/platform/actions/workflows/tests-dashmate.yml) | Passing | Local network, Testnet fullnode, Testnet Evonode |
| Test Suite in browser (batch 2) | Passing | |

### Failing

| Job | Status | Known Issue |
|-----|--------|-------------|
| [Test Suite](https://github.com/dashpay/platform/actions/workflows/tests-test-suite.yml) (`test:suite`) | **Failing** | 2 of 65 tests fail with `bad-txns-inputs-missingorspent` -- a Core-level UTXO rejection during withdrawal tests. Failing since ~Mar 16. |
| [Packages functional tests](https://github.com/dashpay/platform/actions/workflows/tests-packges-functional.yml) | **Failing** | Long-standing flaky failure unrelated to recent code changes. |
| Test Suite in browser (batch 1) | **Cancelled** | Cancelled due to 15-minute timeout. |

### Known Issues

#### Test Suite: `bad-txns-inputs-missingorspent` (since ~Mar 16)

Two withdrawal-related tests fail because Core rejects a transaction whose inputs are missing or already spent. The local network starts and processes blocks normally -- the failure is specific to the withdrawal test scenario.

- **63 tests pass**, 2 fail
- Error: `InvalidRequestError: Transaction is rejected: bad-txns-inputs-missingorspent`
- **Not caused by** the `ssh2`/`nan` compilation warnings (those are non-fatal)
- The failing tests worked on Mar 15 and broke on Mar 16

#### Functional tests: long-standing flakiness

The functional tests (`test/functional`) have been intermittently failing for months. This is a known pre-existing issue unrelated to recent code changes.

## Links

- [All nightly runs](https://github.com/dashpay/platform/actions/workflows/tests.yml?query=event%3Aschedule)
- [Latest nightly run](https://github.com/dashpay/platform/actions/workflows/tests.yml?query=event%3Aschedule+branch%3Av3.1-dev)
- [Long-running Rust nightly](https://github.com/dashpay/platform/actions/workflows/tests-rs-nightly-long-running.yml)
- [Security audits (Rust)](https://github.com/dashpay/platform/actions/workflows/security-audit-rust.yml)
- [Security audits (JS - npm)](https://github.com/dashpay/platform/actions/workflows/security-audit-js-npm.yml)
- [Security audits (JS - CodeQL)](https://github.com/dashpay/platform/actions/workflows/security-audit-js-codeql.yml)
