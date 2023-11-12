const { expect, use } = require('chai');
const sinon = require('sinon');
const chaiAsPromised = require('chai-as-promised');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');
require('setimmediate');

use(chaiAsPromised);
use(sinonChai);
use(dirtyChai);

exports.mochaHooks = {
  beforeEach: () => {
    if (!this.sinon) {
      this.sinon = sinon.createSandbox();
    } else {
      this.sinon.restore();
    }
  },

  afterEach: () => {
    this.sinon.restore();
  },
};

global.expect = expect;
