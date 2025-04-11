const sinon = require('sinon');
const sinonChai = require('sinon-chai');

const { expect, use } = require('chai');
const dirtyChai = require('dirty-chai');

const {
  default: loadWasmDpp,
} = require('@dashevo/wasm-dpp');

use(dirtyChai);
use(sinonChai);

exports.mochaHooks = {
  beforeAll: loadWasmDpp,

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
