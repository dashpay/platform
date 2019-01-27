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

## Evolution helpers
We use [js-evo-services-ctl](https://github.com/dashevo/js-evo-services-ctl) library to manipulate Evolution's services.

## Fixtures

Fixtures are located in [fixtures/](fixtures) directory:
- [blocks.json](fixtures/blocks.json)
- [stateTransitionHeaders.json](fixtures/stateTransitionHeaders.json)

There are several helpers for loading fixtures:
- [getBlockFixtures](../lib/test/fixtures/getBlocksFixture.js)
- [getTransitionHeaderFixtures](../lib/test/fixtures/getStateTransitionsFixture.js)
- [getSTPacketsFixture](../lib/test/fixtures/getSTPacketsFixture.js)

## Other tools

You may find other useful tools for testing in [lib/test](../lib/test) directory.
