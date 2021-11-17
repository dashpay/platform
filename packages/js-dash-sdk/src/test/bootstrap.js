const dotenvSafe = require('dotenv-safe');
const path = require('path');
const { initBlake3 } = require('@dashevo/dpp/lib/util/hash');

const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const { expect, use } = require('chai');
const dirtyChai = require('dirty-chai');

const { initBlake3 } = require('@dashevo/dpp/lib/util/hash');

dotenvSafe.config({
  path: path.resolve(__dirname, '..', '..', '.env'),
});

before(async () => {
  await initBlake3();
});

use(dirtyChai);
use(sinonChai);

// Initialize blake3 hashing for DPP
before(async () => {
  await initBlake3();
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
