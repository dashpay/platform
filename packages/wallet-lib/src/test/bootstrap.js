const { use } = require('chai');
const { expect } = require('chai');
const path = require('path');
const dotenvSafe = require('dotenv-safe');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');

use(sinonChai);
use(dirtyChai);
use(chaiAsPromised);

dotenvSafe.config({
  path: path.resolve(__dirname, '..', '..', '.env'),
});

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
