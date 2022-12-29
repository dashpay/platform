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

const { PrivateKey } = require('@dashevo/dashcore-lib');

use(sinonChai);
use(chaiAsPromised);
use(chaiString);
use(dirtyChai);

process.env.NODE_ENV = 'test';

// Workaround for dotenv-safe
if (process.env.INITIAL_CORE_CHAINLOCKED_HEIGHT === undefined) {
  process.env.INITIAL_CORE_CHAINLOCKED_HEIGHT = 0;
}
if (process.env.DPNS_MASTER_PUBLIC_KEY === undefined) {
  process.env.DPNS_MASTER_PUBLIC_KEY = '037d074eb00aa286c438b5d12b7c6ca25104d61b03e6601b6ace7d5eb036fbbc23';
}
if (process.env.DPNS_SECOND_PUBLIC_KEY === undefined) {
  process.env.DPNS_SECOND_PUBLIC_KEY = '025852df611a228b0e7fbccff4eaa117500ead84622809ea7fc05dcf6d2dbbc1d4';
}
if (process.env.DASHPAY_MASTER_PUBLIC_KEY === undefined) {
  process.env.DASHPAY_MASTER_PUBLIC_KEY = '02c571ff0cdb72634de4fd23f40c4ed530b3d31defc987a55479d65e7e8c1e249a';
}
if (process.env.DASHPAY_SECOND_PUBLIC_KEY === undefined) {
  process.env.DASHPAY_SECOND_PUBLIC_KEY = '03834f92a2132e55273cb713e855a6fbf2179704830c19094470720d6434ce4547';
}
if (process.env.FEATURE_FLAGS_MASTER_PUBLIC_KEY === undefined) {
  process.env.FEATURE_FLAGS_MASTER_PUBLIC_KEY = '022393486382a5bb262856c49f869827a1c79a3a3c38747f3cb8c32dd7bd191797';
}
if (process.env.FEATURE_FLAGS_SECOND_PUBLIC_KEY === undefined) {
  process.env.FEATURE_FLAGS_SECOND_PUBLIC_KEY = '03c10ac08a77dfdfcdc706ea43d9651ac0866181b835411587eca4d2d5477f39f7';
}
if (process.env.MASTERNODE_REWARD_SHARES_MASTER_PUBLIC_KEY === undefined) {
  process.env.MASTERNODE_REWARD_SHARES_MASTER_PUBLIC_KEY = '0333a42c628e8c93ce0386856f8f2239c84bf816cf8590716c7891fdc981a4df0b';
}
if (process.env.MASTERNODE_REWARD_SHARES_SECOND_PUBLIC_KEY === undefined) {
  process.env.MASTERNODE_REWARD_SHARES_SECOND_PUBLIC_KEY = '03a3002856ad91662dc34b6650a2c7f8b2726c947a419e0b880fb3acc38763a271';
}

const dotenvConfig = dotenvSafe.config({
  path: path.resolve(__dirname, '..', '..', '.env'),
});

dotenvExpand(dotenvConfig);

DashCoreOptions.setDefaultCustomOptions({
  container: {
    image: 'dashpay/dashd:18.1.0-rc.1',
  },
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

global.expect = expect;
