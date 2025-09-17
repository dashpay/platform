const chai = require('chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');
const sinonChai = require('sinon-chai');
const sinon = require('sinon');

chai.use(sinonChai);
chai.use(chaiAsPromised);
chai.use(dirtyChai);

const { expect } = chai;
const g = (typeof globalThis !== 'undefined') ? globalThis : global;
g.expect = expect;

exports.mochaHooks = {
  beforeEach() {
    if (!this.sinon) {
      this.sinon = sinon.createSandbox();
    } else {
      this.sinon.restore();
    }
  },

  afterEach() {
    this.sinon.restore();
  },
};

// In browser (Karma), Mocha won't auto-load Node-style root hooks via exports.
// Register them explicitly when mocha global is available.
if (typeof globalThis !== 'undefined' && globalThis.mocha && typeof globalThis.mocha.setup === 'function') {
  globalThis.mocha.setup({ rootHooks: exports.mochaHooks });
}
