import sinon from 'sinon';
import { use } from 'chai';

import dirtyChai from 'dirty-chai';
import sinonChai from 'sinon-chai';

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
