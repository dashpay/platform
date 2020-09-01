const { expect, use } = require('chai');
const sinon = require('sinon');
const chaiAsPromised = require('chai-as-promised');

use(chaiAsPromised);

beforeEach(function beforeEach() {
  if (!this.sinonSandbox) {
    this.sinonSandbox = sinon.createSandbox();
  } else {
    this.sinonSandbox.restore();
  }
});

afterEach(function afterEach() {
  this.sinonSandbox.restore();
});

global.expect = expect;
