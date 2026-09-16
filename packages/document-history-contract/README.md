# Document History Contract

[![Build Status](https://github.com/dashpay/platform/actions/workflows/release.yml/badge.svg)](https://github.com/dashpay/platform/actions/workflows/release.yml)

System data contract recording document transfer, purchase, and price-change
events for document types that opt in via the `keepsTransferHistory`,
`keepsPurchaseHistory`, and `keepsPricingHistory` configuration flags.

History documents are written by the protocol itself during document
transfer, purchase, and price-update state transitions
(`creationRestrictionMode: 2` — they can never be created directly), and are
immutable and permanent once written.

The document types carry provable aggregation trees:

- `purchase` is doctype-averageable on `price` (O(1) provable all-time sale
  count, total volume, and average sale price), and its `byContract` and
  `byDocument` indices are range-averageable over `$createdAt` (provable
  per-contract and per-document sale count / volume / average between dates).
- `priceUpdate` is doctype-countable, and its `byContract` and `byDocument`
  indices are range-averageable over `$createdAt` (provable average asking
  price between dates). These are listing-event-weighted averages, not
  time-weighted prices.
- `transfer` is doctype-countable, and its `byContract` index is
  range-countable over `$createdAt` (provable transfer counts per contract
  and time window).

## Table of Contents

- [Contributing](#contributing)
- [License](#license)

## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashpay/platform/issues/new/choose) or submit PRs.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.
