const sinon = require('sinon');

beforeEach(function beforeEach() {
  if (!this.sinon) {
    this.sinon = sinon.createSandbox();
  } else {
    this.sinon.restore();
  }
});

before(function before() {
  if (!this.sinon) {
    this.sinon = sinon.createSandbox();
  }
});

afterEach(function afterEach() {
  this.sinon.restore();
});
