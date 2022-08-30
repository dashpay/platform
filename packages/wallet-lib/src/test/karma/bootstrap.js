const { expect, use } = require('chai');
const sinon = require('sinon');
const chaiAsPromised = require('chai-as-promised');
const sinonChai = require('sinon-chai');

use(chaiAsPromised);
use(sinonChai);

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
  } else {
    this.sinon.restore();
  }
});

afterEach(function afterEach() {
  this.sinon.restore();
});

after(function before() {
  this.sinon.restore();
});

global.expect = expect;
