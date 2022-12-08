const sinon = require('sinon');
const { expect, use } = require('chai');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');

use(sinonChai);
use(chaiAsPromised);
use(dirtyChai);

process.env.NODE_ENV = 'test';

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
