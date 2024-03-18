const path = require('path');
const dotenvSafe = require('dotenv-safe');
const { expect, use } = require('chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');

use(chaiAsPromised);
use(dirtyChai);
use(sinonChai);

dotenvSafe.config({
  path: path.resolve(__dirname, '..', '..', '.env'),
});

process.env.NODE_ENV = 'test';

let faucetIndex = 1;
if (process.env.MOCHA_WORKER_ID) {
  const mochaWorkerId = parseInt(process.env.MOCHA_WORKER_ID, 10);
  faucetIndex = mochaWorkerId + 1;
}

process.env.FAUCET_ADDRESS = process.env[`FAUCET_${faucetIndex}_ADDRESS`];
process.env.FAUCET_PRIVATE_KEY = process.env[`FAUCET_${faucetIndex}_PRIVATE_KEY`];

exports.mochaHooks = {
  beforeEach() {
    if (!this.sinon) {
      this.sinon = sinon.createSandbox();
    } else {
      this.sinon.restore();
    }
  },

  afterEach() {
    this.sinon.restore();
  },
};

global.expect = expect;
