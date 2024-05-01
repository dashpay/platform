const { use } = require('chai');
const { expect } = require('chai');
const dotenvSafe = require('dotenv-safe');
const path = require('path');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');

use(sinonChai);
use(dirtyChai);
use(chaiAsPromised);

if (process.env.LOAD_ENV === 'true') {
  dotenvSafe.config({
    path: path.resolve(__dirname, '..', '..', '.env'),
  });
}

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
