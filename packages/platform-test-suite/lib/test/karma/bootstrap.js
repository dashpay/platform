const { expect, use } = require('chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
require('setimmediate');

use(chaiAsPromised);
use(dirtyChai);
use(sinonChai);

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
