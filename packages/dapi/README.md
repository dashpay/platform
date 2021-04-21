# DAPI

[![Build Status](https://travis-ci.com/dashevo/dapi.svg?branch=master)](https://travis-ci.com/dashevo/dapi)
[![API stability](https://img.shields.io/badge/stability-stable-green.svg)](https://nodejs.org/api/documentation.html#documentation_stability_index)

> A decentralized API for the Dash network

## Table of Contents
- [Install](#install)
  - [Dependencies](#dependencies)
- [Usage](#usage)
- [Configuration](#configuration)
- [Making requests](#making-basic-requests)
- [API Reference](#api-reference)
- [Contributing](#contributing)
- [License](#license)

## Install

```sh
npm install
```

### Dependencies

DAPI targets the latest LTS release of Node.js. Currently, this is Node v10.13.

DAPI requires the latest version of [dashcore](https://github.com/dashevo/dash-evo-branches/tree/evo) with Evolution features (special branch repo).

1. **Install core.** You can use the docker image (`dashcore:evo`) or clone code from [the repository](https://github.com/dashevo/dash-evo-branches/tree/evo), switch to the `evo` branch, and build it by yourself. Note: you need to build image with ZMQ and wallet support. You can follow the build instructions located [here](https://github.com/dashevo/dash-evo-branches/tree/evo/doc)
2. **Configure core.** DAPI needs dashcore's ZMQ interface to be exposed and all indexes enabled. You can find the example config for dashcore [here](/doc/dependencies_configs/dash.conf). To start dashcore process with this config, copy it somewhere to your system, and then run `./src/dashd -conf=/path/to/your/config`.

## Usage

After you've installed all the dependencies, you can start DAPI by running the `npm start` command inside the DAPI repo directory.

```sh
npm start
```

## Configuration

DAPI is configured via environment variables either explicitly passed or present in the `.env` dotfile. For example, to change the DAPI port, execute DAPI with the following arguments: `RPC_SERVER_PORT=3010 npm start`. Consult the sample environment [file](/.env.example). You can see the full list of available options [here](/doc/CONFIGURATION.md).

## Making basic requests

DAPI uses [JSON-RPC 2.0](https://www.jsonrpc.org/specification) as the main interface. If you want to confirm that DAPI is functioning and synced, you can request the best block height.

Send the following json to your DAPI instance:

```json
{"jsonrpc": "2.0","method": "getBestBlockHeight", "id": 1}
```

Note that you always need to specify an id, otherwise the server will respond with an empty body, as mentioned in the [spec](https://www.jsonrpc.org/specification#notification).

## API Reference

A list of all available RPC commands, along with their various arguments and expected responses can be found [here](/doc/REFERENCE.md)

Implementation of these commands can be viewed [here](/lib/rpcServer/commands).

## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashevo/dapi/issues/new) or submit PRs.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.
