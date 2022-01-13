const path = require('path');
const dotenvSafe = require('dotenv-safe');
const dotenvExpand = require('dotenv-expand');
const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');
const chaiString = require('chai-string');

const { PrivateKey } = require('@dashevo/dashcore-lib');

use(sinonChai);
use(chaiAsPromised);
use(chaiString);
use(dirtyChai);

process.env.NODE_ENV = 'test';

const privateKey = new PrivateKey();

// Workaround for dotenv-safe
if (process.env.INITIAL_CORE_CHAINLOCKED_HEIGHT === undefined) {
  process.env.INITIAL_CORE_CHAINLOCKED_HEIGHT = privateKey.toPublicKey();
}
if (process.env.DPNS_MASTER_PUBLIC_KEY === undefined) {
  process.env.DPNS_MASTER_PUBLIC_KEY = privateKey.toPublicKey();
}
if (process.env.DASHPAY_MASTER_PUBLIC_KEY === undefined) {
  process.env.DASHPAY_MASTER_PUBLIC_KEY = privateKey.toPublicKey();
}
if (process.env.FEATURE_FLAGS_MASTER_PUBLIC_KEY === undefined) {
  process.env.FEATURE_FLAGS_MASTER_PUBLIC_KEY = privateKey.toPublicKey();
}
if (process.env.MASTERNODE_REWARD_SHARES_MASTER_PUBLIC_KEY === undefined) {
  process.env.MASTERNODE_REWARD_SHARES_MASTER_PUBLIC_KEY = privateKey.toPublicKey();
}

const dotenvConfig = dotenvSafe.config({
  path: path.resolve(__dirname, '..', '..', '.env'),
});
dotenvExpand(dotenvConfig);

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

global.expect = expect;
