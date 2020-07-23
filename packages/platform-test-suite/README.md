# Dash Platform Test Suite

> The test suite for end-to-end and functional testing the Dash Platform by running some real-life scenarios against a Dash Network

## Table of Contents
- [Pre-Requisites](#pre-requisites)
- [Usage](#usage)
- [Contributing](#contributing)
- [License](#license)

## Pre-requisites

You may run test-suite against any platform compatible network, or even [local node](https://github.com/dashevo/mn-bootstrap).
To run locally make sure you have [Node.js](https://nodejs.org/) installed.
To run using [Docker](https://www.docker.com/), make sure you have it installed.

## Usage

### Running locally

Install all the necessary dependencies:

```sh
$ npm i
```

Use `./bin/test.sh` script to run tests:

```sh
$ ./bin/test.sh

Run test suite

Usage: test <seed> [options]

  <seed> can be IP or IP:port

  Options:
              --npm-install=pkg                             - install npm package before running the suite
  -s=a,b,c    --scope=a,b,c                                 - test scope to run
  -k=key      --faucet-key=key                              - faucet private key string
  -n=network  --network=network                             - use regtest or testnet
              --dpns-tld-identity-private-key=private_key   - top level identity private key
              --dpns-tld-identity-id=identity_id            - top level identity id
              --dpns-contract-id=contract_id                - dpns contract id
  -h          --help                                        - show help

  Possible scopes:
  e2e
  functional
  core
  platform
  e2e:dpns
  e2e:contacts
  functional:core
  functional:platform
```

### Running using Docker

Just run pre-built image using the same arguments as [running locally](#running-locally):

```sh
$ docker run --network=host dashpay/platform-test-suite

Run test suite

Usage: test <seed> [options]

  <seed> can be IP or IP:port

  Options:
              --npm-install=pkg                             - install npm package before running the suite
  -s=a,b,c    --scope=a,b,c                                 - test scope to run
  -k=key      --faucet-key=key                              - faucet private key string
  -n=network  --network=network                             - use regtest or testnet
              --dpns-tld-identity-private-key=private_key   - top level identity private key
              --dpns-tld-identity-id=identity_id            - top level identity id
              --dpns-contract-id=contract_id                - dpns contract id
  -h          --help                                        - show help

  Possible scopes:
  e2e
  functional
  core
  platform
  e2e:dpns
  e2e:contacts
  functional:core
  functional:platform
```

## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashevo/platform-test-suite/issues/new) or submit PRs.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.
