const path = require('path');
const dotenvSafe = require('dotenv-safe');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const { use } = require('chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');

if (process.env.LOAD_ENV === 'true') {
  dotenvSafe.config({
    path: path.resolve(__dirname, '..', '..', '.env'),
  });
}

use(dirtyChai);
use(sinonChai);
use(chaiAsPromised);

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
