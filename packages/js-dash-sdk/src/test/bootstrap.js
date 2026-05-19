import path from 'path';
import { fileURLToPath } from 'url';
import dotenvSafe from 'dotenv-safe';
import sinon from 'sinon';
import sinonChai from 'sinon-chai';
import { use } from 'chai';
import dirtyChai from 'dirty-chai';
import chaiAsPromised from 'chai-as-promised';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

if (process.env.LOAD_ENV === 'true') {
  dotenvSafe.config({
    path: path.resolve(__dirname, '..', '..', '.env'),
  });
}

use(dirtyChai);
use(sinonChai);
use(chaiAsPromised);

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
