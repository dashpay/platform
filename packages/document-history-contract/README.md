# Document History Contract

[![Build Status](https://github.com/dashpay/platform/actions/workflows/release.yml/badge.svg)](https://github.com/dashpay/platform/actions/workflows/release.yml)

System data contract recording document transfer, purchase, and price-change
events for document types that opt in via the `keepsTransferHistory`,
`keepsPurchaseHistory`, and `keepsPricingHistory` configuration flags.

History documents are written by the protocol itself during document
transfer, purchase, and price-update state transitions
(`creationRestrictionMode: 2` — they can never be created directly), and are
immutable and permanent once written.

## Table of Contents

- [Contributing](#contributing)
- [License](#license)

## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashpay/platform/issues/new/choose) or submit PRs.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.
