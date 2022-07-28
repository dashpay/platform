# Dash SDK

[![NPM Version](https://img.shields.io/npm/v/dash)](https://www.npmjs.org/package/dash)
[![Release Packages](https://github.com/dashevo/platform/actions/workflows/release.yml/badge.svg)](https://github.com/dashevo/platform/actions/workflows/release.yml)
[![Release Date](https://img.shields.io/github/release-date/dashevo/platform)](https://github.com/dashevo/platform/releases/latest)
[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen)](https://github.com/RichardLitt/standard-readme)

Dash library for JavaScript/TypeScript ecosystem (Wallet, DAPI, Primitives, BLS, ...)

Dash library provides access via [DAPI](https://dashplatform.readme.io/docs/explanation-dapi) to use both the Dash Core network and Dash Platform on [supported networks](https://github.com/dashevo/platform/#supported-networks). The Dash Core network can be used to broadcast and receive payments. Dash Platform can be used to manage identities, register data contracts for applications, and submit or retrieve application data via documents.

## Install

### From NPM
In order to use this library, you will need to add our [NPM package](https://www.npmjs.com/dash) to your project.

Having [NodeJS](https://nodejs.org/) installed, just type:

```bash
npm install dash
```

### From unpkg
```html
<script src="https://unpkg.com/dash"></script>
```

### Usage examples

- [Generate a mnemonic](examples/generate-a-new-mnemonic.md)
- [Receive money and display balance](examples/receive-money-and-check-balance.md)
- [Pay to another address](examples/pay-to-another-address.md)
- [Use another BIP44 account](examples/use-different-account.md)

### Dash Platform Tutorials

See the [Tutorial section](https://dashplatform.readme.io/docs/tutorials-introduction) of the Dash Platform documentation for examples.

## Licence

[MIT](https://github.com/dashevo/dashjs/blob/master/LICENCE.md) © Dash Core Group, Inc.
