const path = require('path');
const dotenv = require('dotenv');
const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');

use(sinonChai);
use(dirtyChai);
use(chaiAsPromised);

process.env.NODE_ENV = 'test';

dotenv.config({
  path: path.resolve(__dirname, '..', '..', '.env'),
});

beforeEach(function beforeEach() {
  if (!this.sinon) {
    this.sinon = sinon.sandbox.create();
  } else {
    this.sinon.restore();
  }
});

global.expect = expect;
