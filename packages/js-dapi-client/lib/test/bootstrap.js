import chai from 'chai';
import sinon from 'sinon';
import sinonChai from 'sinon-chai';
import dirtyChai from 'dirty-chai';
import chaiAsPromised from 'chai-as-promised';
import loadDpp from '@dashevo/wasm-dpp';

chai.use(sinonChai);
chai.use(chaiAsPromised);
chai.use(dirtyChai);

export const mochaHooks = {
  // wasm-dpp is CJS; under NodeNext the default may resolve to the namespace
  // object instead of the callable. Unwrap defensively.
  beforeAll: async () => {
    const load = loadDpp.default ?? loadDpp;
    return load();
  },

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

global.expect = chai.expect;
