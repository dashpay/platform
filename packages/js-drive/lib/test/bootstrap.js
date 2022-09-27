const path = require('path');
const dotenvSafe = require('dotenv-safe');
const dotenvExpand = require('dotenv-expand');
const { expect, use } = require('chai');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');
const chaiString = require('chai-string');
const DashCoreOptions = require('@dashevo/dp-services-ctl/lib/services/dashCore/DashCoreOptions');

use(sinonChai);
use(chaiAsPromised);
use(chaiString);
use(dirtyChai);

process.env.NODE_ENV = 'test';

const testPublicKey = '029470f30d543c500558080bf953f96f4beda8ce3c7e00965891913e586f682bb4';

// Workaround for dotenv-safe
if (process.env.INITIAL_CORE_CHAINLOCKED_HEIGHT === undefined) {
  process.env.INITIAL_CORE_CHAINLOCKED_HEIGHT = testPublicKey;
}
if (process.env.DPNS_MASTER_PUBLIC_KEY === undefined) {
  process.env.DPNS_MASTER_PUBLIC_KEY = testPublicKey;
}
if (process.env.DPNS_SECOND_PUBLIC_KEY === undefined) {
  process.env.DPNS_SECOND_PUBLIC_KEY = testPublicKey;
}
if (process.env.DASHPAY_MASTER_PUBLIC_KEY === undefined) {
  process.env.DASHPAY_MASTER_PUBLIC_KEY = testPublicKey;
}
if (process.env.DASHPAY_SECOND_PUBLIC_KEY === undefined) {
  process.env.DASHPAY_SECOND_PUBLIC_KEY = testPublicKey;
}
if (process.env.FEATURE_FLAGS_MASTER_PUBLIC_KEY === undefined) {
  process.env.FEATURE_FLAGS_MASTER_PUBLIC_KEY = testPublicKey;
}
if (process.env.FEATURE_FLAGS_SECOND_PUBLIC_KEY === undefined) {
  process.env.FEATURE_FLAGS_SECOND_PUBLIC_KEY = testPublicKey;
}
if (process.env.MASTERNODE_REWARD_SHARES_MASTER_PUBLIC_KEY === undefined) {
  process.env.MASTERNODE_REWARD_SHARES_MASTER_PUBLIC_KEY = testPublicKey;
}
if (process.env.MASTERNODE_REWARD_SHARES_SECOND_PUBLIC_KEY === undefined) {
  process.env.MASTERNODE_REWARD_SHARES_SECOND_PUBLIC_KEY = testPublicKey;
}

const dotenvConfig = dotenvSafe.config({
  path: path.resolve(__dirname, '..', '..', '.env'),
});

dotenvExpand(dotenvConfig);

if (process.env.SERVICE_IMAGE_CORE) {
  DashCoreOptions.setDefaultCustomOptions({
    container: {
      image: 'dashpay/dashd:18.1.0-rc.1',
    },
  });
}

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
