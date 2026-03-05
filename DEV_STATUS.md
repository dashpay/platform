# Dev Status

This page tracks the health of nightly checks and security audits for the Dash Platform repository.

## Security Audits

These audits run nightly (4:30 AM UTC) and can also be triggered manually.

| Audit | Status |
|-------|--------|
| Rust Crates Security | [![Rust crates security audit](https://github.com/dashpay/platform/actions/workflows/security-audits.yml/badge.svg)](https://github.com/dashpay/platform/actions/workflows/security-audits.yml) |
| JS NPM Security | [![JS NPM security audit](https://github.com/dashpay/platform/actions/workflows/security-audits.yml/badge.svg)](https://github.com/dashpay/platform/actions/workflows/security-audits.yml) |
| JS CodeQL Analysis | [![JS code security audit](https://github.com/dashpay/platform/actions/workflows/security-audits.yml/badge.svg)](https://github.com/dashpay/platform/actions/workflows/security-audits.yml) |

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
