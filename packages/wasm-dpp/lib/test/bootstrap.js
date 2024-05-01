const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');
const chaiString = require('chai-string');
const chaiExclude = require('chai-exclude');
const crypto = require('crypto');
const { default: loadDpp } = require('@dashevo/wasm-dpp');

use(sinonChai);
use(chaiAsPromised);
use(dirtyChai);
use(chaiString);
use(chaiExclude);

/* eslint-disable */
// TODO this should be loaded with library - not with tests.
globalThis.crypto = crypto.webcrypto;

exports.mochaHooks = {
  beforeAll: loadDpp,

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
