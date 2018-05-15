# Dash Drive Tests

We believe in [Test Pyramid](http://verraes.net/2015/01/economy-of-tests/).

## Structure

 - `e2e/` - [End-to-end (e2e, system) tests](https://en.wikipedia.org/wiki/System_testing)  
 - `integration/` - [Integration tests](https://en.wikipedia.org/wiki/Integration_testing)
 - `unit/` - [Unit tests](https://en.wikipedia.org/wiki/Unit_testing)

A subsequent paths the same as the structure of the code in the [lib/](../lib) directory.

## How to run tests

Run all tests:

```bash
npm test
```

Run unit tests:

```bash
npm run test:unit
```

Run integration tests:

```bash
npm run test:integration
```

Run e2e tests:

```bash
npm run test:e2e
```

## How to write tests

We use:
 - [Mocha](https://mochajs.org) as testing framework
 - [Sinon.JS](http://sinonjs.org/) for stubs and spies
 - [Chai](http://chaijs.com/) with several plugins for assertions:
   - [Sinon Chai](https://github.com/domenic/sinon-chai) for Sinon.JS assertions
   - [Chai as promised](https://github.com/domenic/chai-as-promised) for assertions about promises
   - [Dirty Chai](https://github.com/prodatakey/dirty-chai) for lint-friendly terminating assertions

We prefer `expect` assertions syntax instead of `should`.

All tools are [bootstrapped](../lib/test/bootstrap.js) before tests:
 - `expect` function is available in global context
 - Sinon sandbox is created before each test and available as `this.sinon` property in the test's context
 - Envs from `.env` are loaded before all tests

## Helpers

### Start IPFS

```js
const startIPFSInstance = require('../lib/test/services/mocha/startIPFSInstance');

let ipfsApi;
startIPFSInstance().then((instance) => {
  ipfsApi = instance;
});
```

Use `many` method to start several IPFS instances:

```js
const startIPFSInstance = require('../lib/test/services/mocha/startIPFSInstance');

let ipfsApi1;
let ipfsApi2;
startIPFSInstance.many(2).then((instances) => {
  [ipfsApi1, ipfsApi2] = instances;
});
```

 - `startIPFSInstance` returns instance of [IpfsApi](https://github.com/ipfs/js-ipfs-api#api)
 - IPFS storage is cleaned up before each test
 - IPFS is stopped when tests are over

### Start Dash Core

```js
const startDashCoreInstance = require('../lib/test/services/mocha/startDashCoreInstance');

let dashCoreInstance;
startDashCoreInstance().then((instance) => {
  dashCoreInstance = instance;
});
```

 - Use `many` method to start several Dash Core instances
 - `startDashCoreInstance` returns instance of [DashCoreInstance](../lib/test/services/dashCore/DashCoreInstance.js)
 - Dash Core is restarted and data are cleaned up before each test
 - Dash Core is removed when tests are over
 
### Start Dash Drive

```js
const startDashDriveInstance = require('../lib/test/services/mocha/startDashDriveInstance');

let dashDriveInstance;
startDashDriveInstance().then((instance) => {
  dashDriveInstance = instance;
});
```

- Use `many` method to start several Dash Drive instances
- `startDashDriveInstance` returns an instance of:
  - [ipfs](https://github.com/ipfs/js-ipfs-api#api)
  - [dashCore](../lib/test/services/dashCore/DashCoreInstance.js)
  - [dashDrive](../lib/test/services/docker/DockerInstance.js)
  - [mongoDb](../lib/test/services/mongoDb/MongoDbInstance.js)
- Dash Drive process is restarted and data are cleaned up before each test
- Dash Drive is removed when tests are over

### Start MongoDB

```js
const startMongoDbInstance = require('../lib/test/services/mocha/startMongoDbInstance');

let mongoDb;
startMongoDbInstance().then((instance) => {
  mongoDb = instance;
});
```

- Use `many` method to start several MongoDb instances
- `startMongoDbInstance` returns instance of [MongoDbInstance](../lib/test/services/mongoDb/MongoDbInstance.js)
- MongoDB is cleaned up after each tests
- MongoDB is removed when tests are ove

## Fixtures

Fixtures are located in [fixtures/](fixtures) directory:
- [blocks.json](fixtures/blocks.json)
- [stateTransitionHeaders.json](fixtures/stateTransitionHeaders.json)
- [stateTransitionPackets.json](fixtures/stateTransitionPackets.json)

There are several helpers for loading fixtures:
- [getBlockFixtures](../lib/test/fixtures/getBlockFixtures.js)
- [getTransitionHeaderFixtures](../lib/test/fixtures/getTransitionHeaderFixtures.js)

## Other tools

You may find other useful tools for testing in [lib/test](../lib/test) directory.
