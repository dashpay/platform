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
