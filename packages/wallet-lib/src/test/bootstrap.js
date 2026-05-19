import { use, expect } from 'chai';
import dotenvSafe from 'dotenv-safe';
import path from 'path';
import sinon from 'sinon';
import sinonChai from 'sinon-chai';
import dirtyChai from 'dirty-chai';
import chaiAsPromised from 'chai-as-promised';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

use(sinonChai);
use(dirtyChai);
use(chaiAsPromised);

if (process.env.LOAD_ENV === 'true') {
  dotenvSafe.config({
    path: path.resolve(__dirname, '..', '..', '.env'),
  });
}

export const mochaHooks = {
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

global.expect = expect;
