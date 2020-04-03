# Drive

[![Build Status](https://travis-ci.com/dashevo/drive.svg?branch=master)](https://travis-ci.com/dashevo/drive)

> Replicated state machine for Dash Platform

Drive is the storage component of Dash Platform, allowing developers to store and secure their application data through Dash's masternode network. Application data structures are defined by a data contract, which is stored on Drive and used to verify/validate updates to your application data.

## Table of Contents
- [Install](#install)
- [Usage](#usage)
- [Configuration](#configuration)
- [Tests](#tests)
- [Maintainer](#maintainer)
- [Contributing](#contributing)
- [License](#license)

## Install

1. [Install Node.JS 12 and higher](https://nodejs.org/en/download/)
2. Copy `.env.example` to `.env` file
3. Install npm dependencies: `npm install`

## Usage

```bash
npm run abci
```

## Configuration

Drive uses environment variables for configuration.
Variables are read from `.env` file and can be overwritten by variables
defined in env or directly passed to the process.

See all available settings in [.env.example](.env.example).

## Tests

[Read](test/) about tests in `test/` folder.

## Maintainer

[@shumkov](https://github.com/shumkov)

## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashevo/drive/issues/new) or submit PRs.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.
