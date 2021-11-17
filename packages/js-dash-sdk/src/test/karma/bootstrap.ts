import sinon from 'sinon';
import { use } from 'chai';

import dirtyChai from 'dirty-chai';
import sinonChai from 'sinon-chai';

// @ts-ignore
const { initBlake3 } = require('@dashevo/dpp/lib/util/hash');

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
