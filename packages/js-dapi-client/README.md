# DAPI Client

[![Build Status](https://travis-ci.com/dashevo/dapi-client.svg?branch=master)](https://travis-ci.com/dashevo/dapi-client)

> Client library used to access Dash DAPI endpoints

## Table of Contents
- [Install](#install)
- [Usage](#usage)
- [Contributing](#contributing)
- [License](#license)

## Install

```sh
npm install @dashevo/dapi-client
```

## Usage

```javascript
const DAPIClient = require('@dashevo/dapi-client');
var client = new DAPIClient();

client.getBalance('testaddress');
```

## Contributing

Feel free to dive in! [Open an issue](https://github.com/dashevo/dapi-client/issues/new) or submit PRs.

## License

[MIT](LICENSE) &copy; Dash Core Group, Inc.
