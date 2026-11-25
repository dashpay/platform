# DPNS Contract

[![Build Status](https://github.com/dashpay/platform/actions/workflows/release.yml/badge.svg)](https://github.com/dashpay/platform/actions/workflows/release.yml)
[![NPM version](https://img.shields.io/npm/v/@dashevo/dpns-contract.svg?style=flat-square)](https://npmjs.org/package/@dashevo/dpns-contract)

JSON Contracts for Dash Platform Name Service

## Table of Contents

- [Install](#install)
- [Usage](#usage)
- [Giving up a name](#giving-up-a-name)
- [Contributing](#contributing)
- [License](#license)

## Install

```sh
npm install @dashevo/dpns-contract
```

## Usage

```sh
# TODO ...
```

## Giving up a name

From contract v3 (protocol version 15) the identity holding a name can throw it away, in one of two ways:

- **Delete** the `domain` document. The name is free again and anybody may register it. A contested name (a normalized label of 3 to 19 `a-z`, `0`, `1` or `-` characters, awarded by a masternode vote) can not be deleted.
- **Tombstone** it: create a `tombstone` document holding the name's `normalizedLabel` and `normalizedParentDomainName`. The create deletes the creator's own `domain` of that name, and nobody can register the name while the tombstone exists. Any name can be tombstoned, a contested one included.

Nobody can replace, delete or transfer a tombstone, the identity that created it included. A tombstoned name may later be brought back and redistributed through a direct or indirect will of the DAO.

## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashpay/platform/issues/new/choose) or submit PRs.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.
