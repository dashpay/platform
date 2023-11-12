require('../../../polyfills/fetch-polyfill');
require('setimmediate');

const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');
const { default: loadDpp } = require('@dashevo/wasm-dpp');

use(sinonChai);
use(chaiAsPromised);
use(dirtyChai);

before(async () => {
  await loadDpp();
});

beforeEach(function beforeEach() {
  if (!this.sinon) {
    this.sinon = sinon.createSandbox();
  } else {
    this.sinon.restore();
  }
});

afterEach(function afterEach() {
  this.sinon.restore();
});

before(function before() {
  if (!this.sinon) {
    this.sinon = sinon.createSandbox();
  }
});

global.expect = expect;
