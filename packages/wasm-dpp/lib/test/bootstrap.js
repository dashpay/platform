const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');
const chaiString = require('chai-string');
const chaiExclude = require('chai-exclude');

use(sinonChai);
use(chaiAsPromised);
use(dirtyChai);
use(chaiString);
use(chaiExclude);

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
