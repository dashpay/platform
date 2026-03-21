# Nightly Test Status

[![Nightly Tests](https://github.com/dashpay/platform/actions/workflows/tests.yml/badge.svg?event=schedule)](https://github.com/dashpay/platform/actions/workflows/tests.yml?query=event%3Aschedule)

> **Note:** This page is manually maintained. For live results, check the [latest nightly run](https://github.com/dashpay/platform/actions/workflows/tests.yml?query=event%3Aschedule+branch%3Av3.1-dev) directly.

Nightly tests run every day at **23:00 UTC** on the `v3.1-dev` branch via the [Tests workflow](https://github.com/dashpay/platform/actions/workflows/tests.yml?query=event%3Aschedule). They exercise the full CI pipeline including Docker image builds, E2E tests, and the platform test suite.

## Nightly Jobs

### Always run on schedule

These jobs have `github.event_name == 'schedule'` in their conditions and run every night.

| Job | Status | Notes |
|-----|--------|-------|
| [Build JS packages](https://github.com/dashpay/platform/actions/workflows/tests-build-js.yml) | Passing | |
| [Build Docker images](https://github.com/dashpay/platform/actions/workflows/tests-build-image.yml) (Drive, RS-DAPI, Dashmate helper) | Passing | All 3 images build successfully |
| [JS dependency versions](https://github.com/dashpay/platform/actions/workflows/tests.yml) | Passing | `yarn constraints` check |
| [Dashmate E2E tests](https://github.com/dashpay/platform/actions/workflows/tests-dashmate.yml) | Passing | Local network, Testnet fullnode, Testnet Evonode |
| [Test Suite](https://github.com/dashpay/platform/actions/workflows/tests-test-suite.yml) (`test:suite`) | **Failing** | 2 of 65 tests fail with `bad-txns-inputs-missingorspent`. See [known issues](#test-suite-bad-txns-inputs-missingorspent-since-mar-16). |
| [Test Suite in browser (1)](https://github.com/dashpay/platform/actions/workflows/tests-test-suite.yml) | **Cancelled** | Cancelled due to 15-minute timeout. |
| [Test Suite in browser (2)](https://github.com/dashpay/platform/actions/workflows/tests-test-suite.yml) | Passing | |
| [Packages functional tests](https://github.com/dashpay/platform/actions/workflows/tests-packges-functional.yml) | **Failing** | Long-standing flaky failure. See [known issues](#functional-tests-long-standing-flakiness). |

### Conditional (run when changes detected)

These jobs only run on nightly if relevant files changed in the latest commit. They may be skipped entirely on a given night.

| Job | Status | Condition |
|-----|--------|-----------|
| [Rust workspace tests](https://github.com/dashpay/platform/actions/workflows/tests-rs-workspace.yml) (macOS) | Passing (when run) | Rust package changes |
| [Swift SDK build](https://github.com/dashpay/platform/actions/workflows/swift-sdk-build.yml) | Passing (when run) | swift-sdk, rs-sdk, or rs-sdk-ffi changes |
| [JS package tests](https://github.com/dashpay/platform/actions/workflows/tests-js-package.yml) | Passing (when run) | JS package changes |

## Known Issues

### Test Suite: `bad-txns-inputs-missingorspent` (since ~Mar 16)

Two withdrawal-related tests fail because Core rejects a transaction whose inputs are missing or already spent. The local network starts and processes blocks normally -- the failure is specific to the withdrawal test scenario.

- **63 tests pass**, 2 fail
- Error: `InvalidRequestError: Transaction is rejected: bad-txns-inputs-missingorspent`
- **Not caused by** the `ssh2`/`nan` compilation warnings (those are non-fatal)

### Functional tests: long-standing flakiness

The functional tests have been intermittently failing for months. This is a known pre-existing issue unrelated to recent code changes.

## Links

- [All nightly runs](https://github.com/dashpay/platform/actions/workflows/tests.yml?query=event%3Aschedule)
- [Latest nightly run](https://github.com/dashpay/platform/actions/workflows/tests.yml?query=event%3Aschedule+branch%3Av3.1-dev)
- [Long-running Rust nightly](https://github.com/dashpay/platform/actions/workflows/tests-rs-nightly-long-running.yml)
- [Security audits (Rust)](https://github.com/dashpay/platform/actions/workflows/security-audit-rust.yml)
- [Security audits (JS - npm)](https://github.com/dashpay/platform/actions/workflows/security-audit-js-npm.yml)
- [Security audits (JS - CodeQL)](https://github.com/dashpay/platform/actions/workflows/security-audit-js-codeql.yml)
