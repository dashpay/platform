const { expect, use } = require('chai');
const sinon = require('sinon');
const chaiAsPromised = require('chai-as-promised');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');
require('setimmediate');

use(chaiAsPromised);
use(sinonChai);
use(dirtyChai);

beforeEach(function beforeEach() {
  if (!this.sinon) {
    this.sinon = sinon.createSandbox();
    // Legacy
    this.sinonSanbox = this.sinon;
  } else {
    this.sinon.restore();
  }
});

before(function before() {
  if (!this.sinon) {
    this.sinon = sinon.createSandbox();
  }
});

afterEach(function afterEach() {
  this.sinon.restore();
});

global.expect = expect;
