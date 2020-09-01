const path = require('path');
const dotenvSafe = require('dotenv-safe');
const sinon = require('sinon');

dotenvSafe.config({
  path: path.resolve(__dirname, '..', '..', '.env'),
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
