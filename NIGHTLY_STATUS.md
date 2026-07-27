# Nightly Test Status

[![Nightly Tests](https://github.com/dashpay/platform/actions/workflows/tests.yml/badge.svg?event=schedule)](https://github.com/dashpay/platform/actions/workflows/tests.yml?query=event%3Aschedule)

> **Note:** This page is manually maintained. For live results, check the [latest nightly run](https://github.com/dashpay/platform/actions/workflows/tests.yml?query=event%3Aschedule+branch%3Av4.1-dev) directly.

Nightly tests run every day at **23:00 UTC** on the `v4.1-dev` branch via the [Tests workflow](https://github.com/dashpay/platform/actions/workflows/tests.yml?query=event%3Aschedule). They exercise the full CI pipeline including Docker image builds, E2E tests, and the platform test suite.

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

Seven tests fail because Core rejects faucet wallet funding transactions whose inputs are already in the mempool. The failures are in the Data Contract and Contacts test groups -- 1 `before all` hook failure cascades into 6 dependent Contacts tests.

- **65 tests pass**, 7 fail (1 Data Contract funding + 6 Contacts cascade)
- Error: `InvalidRequestError: Transaction is rejected: bad-txns-inputs-missingorspent`
- **Root cause:** The wallet-lib retry logic at `broadcastTransaction.js:181` checks for `'invalid transaction: bad-txns-inputs-missingorspent'` but DAPI returns `'Transaction is rejected: bad-txns-inputs-missingorspent'` -- the retry never matches, so UTXO conflicts are not retried.
- **Not caused by** the `ssh2`/`nan` compilation warnings (those are non-fatal)
- **Fix:** PR #3434 updates the check to use `.includes('bad-txns-inputs-missingorspent')`

### Functional tests: long-standing flakiness

The functional tests have been intermittently failing for months. This is a known pre-existing issue unrelated to recent code changes.

## Links

- [All nightly runs](https://github.com/dashpay/platform/actions/workflows/tests.yml?query=event%3Aschedule)
- [Latest nightly run](https://github.com/dashpay/platform/actions/workflows/tests.yml?query=event%3Aschedule+branch%3Av4.1-dev)
- [Long-running Rust nightly](https://github.com/dashpay/platform/actions/workflows/tests-rs-nightly-long-running.yml)
- [Security audits (Rust)](https://github.com/dashpay/platform/actions/workflows/security-audit-rust.yml)
- [Security audits (JS - npm)](https://github.com/dashpay/platform/actions/workflows/security-audit-js-npm.yml)
- [Security audits (JS - CodeQL)](https://github.com/dashpay/platform/actions/workflows/security-audit-js-codeql.yml)
