# Dev Status

This page tracks the health of nightly builds, integration tests, and security audits for the Dash Platform repository.

## Nightly Builds

The full test suite runs nightly at 11:00 PM UTC, including Docker image builds and integration tests.

| Component | Status |
|-----------|--------|
| Tests (overall) | [![Tests](https://github.com/dashpay/platform/actions/workflows/tests.yml/badge.svg?event=schedule)](https://github.com/dashpay/platform/actions/workflows/tests.yml?query=event%3Aschedule) |

### Docker Images

Docker images are built nightly and pushed to ECR, tagged by commit SHA.

| Image | Target |
|-------|--------|
| `drive` | `drive-abci` |
| `rs-dapi` | `rs-dapi` |
| `dashmate-helper` | `dashmate-helper` |

Images are available at: `<AWS_ACCOUNT_ID>.dkr.ecr.<AWS_REGION>.amazonaws.com/<image>:sha-<commit>`

To pull the latest nightly image, find the commit SHA from the most recent [scheduled Tests run](https://github.com/dashpay/platform/actions/workflows/tests.yml?query=event%3Aschedule).

## Security Audits

These audits run nightly (11:30 PM UTC) and can also be triggered manually.

| Audit | Status |
|-------|--------|
| Rust Crates Security | [![Security: Rust Crates](https://github.com/dashpay/platform/actions/workflows/security-audit-rust.yml/badge.svg)](https://github.com/dashpay/platform/actions/workflows/security-audit-rust.yml) |
| JS NPM Security | [![Security: JS NPM](https://github.com/dashpay/platform/actions/workflows/security-audit-js-npm.yml/badge.svg)](https://github.com/dashpay/platform/actions/workflows/security-audit-js-npm.yml) |
| JS CodeQL Analysis | [![Security: JS CodeQL](https://github.com/dashpay/platform/actions/workflows/security-audit-js-codeql.yml/badge.svg)](https://github.com/dashpay/platform/actions/workflows/security-audit-js-codeql.yml) |

## How to investigate failures

1. Click the badge link for the failing check
2. Open the failed workflow run in GitHub Actions
3. Check the job logs for details
4. For Rust advisories, see [RustSec Advisory Database](https://rustsec.org/advisories/)
5. For NPM advisories, see [GitHub Advisory Database](https://github.com/advisories)
