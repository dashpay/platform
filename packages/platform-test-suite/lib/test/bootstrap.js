import path from 'path';
import { fileURLToPath } from 'url';
import dotenvSafe from 'dotenv-safe';
import { expect, use } from 'chai';
import dirtyChai from 'dirty-chai';
import chaiAsPromised from 'chai-as-promised';
import sinon from 'sinon';
import sinonChai from 'sinon-chai';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

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

export const mochaHooks = {
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
