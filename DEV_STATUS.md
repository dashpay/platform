# Dev Status

This page tracks the health of nightly checks and security audits for the Dash Platform repository.

## Security Audits

These audits run nightly (11:30 PM UTC) and can also be triggered manually.

| Audit | Status |
|-------|--------|
| Rust Crates Security | [![Security: Rust Crates](https://github.com/dashpay/platform/actions/workflows/security-audit-rust.yml/badge.svg)](https://github.com/dashpay/platform/actions/workflows/security-audit-rust.yml) |
| JS NPM Security | [![Security: JS NPM](https://github.com/dashpay/platform/actions/workflows/security-audit-js-npm.yml/badge.svg)](https://github.com/dashpay/platform/actions/workflows/security-audit-js-npm.yml) |
| JS CodeQL Analysis | [![Security: JS CodeQL](https://github.com/dashpay/platform/actions/workflows/security-audit-js-codeql.yml/badge.svg)](https://github.com/dashpay/platform/actions/workflows/security-audit-js-codeql.yml) |

## CI

| Check | Status |
|-------|--------|
| Tests | [![Tests](https://github.com/dashpay/platform/actions/workflows/tests.yml/badge.svg)](https://github.com/dashpay/platform/actions/workflows/tests.yml) |

## How to investigate failures

1. Click the badge link for the failing audit
2. Open the failed workflow run in GitHub Actions
3. Check the job logs for details on which dependency or code pattern triggered the alert
4. For Rust advisories, see [RustSec Advisory Database](https://rustsec.org/advisories/)
5. For NPM advisories, see [GitHub Advisory Database](https://github.com/advisories)
