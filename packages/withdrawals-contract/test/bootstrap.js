const sinon = require('sinon');
const sinonChai = require('sinon-chai');

const { expect, use } = require('chai');
const dirtyChai = require('dirty-chai');

use(dirtyChai);
use(sinonChai);

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

global.expect = expect;
