# Dash Network end-to-end tests

> The test suite for end-to-end testing the Dash Platform by running some real-life scenarios against a Dash Network

## Table of Contents
- [Pre-Requisites](#pre-requisites)
- [Install](#install)
- [Usage](#usage)
- [Configuration](#configuration)
- [Contributing](#contributing)
- [License](#license)

## Pre-requisites

A testnet or devnet should be running. If not you can deploy your own network with [dash-network-deploy](https://github.com/dashpay/dash-network-deploy).

## Install

```sh
npm install
```

## Usage

Run the tests

```sh
npm test
```

## Configuration

Configure DAPI Client seeds and port in `.env` file. Use [.env.example](https://github.com/dashpay/dash-network-e2e-tests/blob/master/.env.example) as an example.

## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashevo/dash-network-e2e-tests/issues/new) or submit PRs.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.
