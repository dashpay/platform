const path = require('path');
const dotenvSafe = require('dotenv-safe');
const sinon = require('sinon');

const { initBlake3 } = require('@dashevo/dpp/lib/util/hash');

dotenvSafe.config({
  path: path.resolve(__dirname, '..', '..', '.env'),
});

before(async () => {
  await initBlake3();
});

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
