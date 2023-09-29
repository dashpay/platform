const dotenv = require('dotenv');
const path = require('path');

const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const { use } = require('chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');

dotenv.config({
  path: path.resolve(__dirname, '..', '..', '.env'),
});

use(dirtyChai);
use(sinonChai);
use(chaiAsPromised);

before(function before() {
  if (!this.sinon) {
    this.sinon = sinon.createSandbox();
  } else {
    this.sinon.restore();
  }
});

after(function after() {
  this.sinon.restore();
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
