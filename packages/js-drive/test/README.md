# Drive Tests

We believe in [Test Pyramid](http://verraes.net/2015/01/economy-of-tests/).

## Structure

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

## Other tools

You may find other useful tools for testing in [lib/test](../lib/test) directory.
