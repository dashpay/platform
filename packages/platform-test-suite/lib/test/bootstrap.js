const path = require('path');
const dotenvSafe = require('dotenv-safe');
const { expect, use } = require('chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');
const Dash = require('dash');

use(chaiAsPromised);
use(dirtyChai);

process.env.NODE_ENV = 'test';

dotenvSafe.config({
  path: path.resolve(__dirname, '..', '..', '.env'),
});

global.expect = expect;

const seeds = [{ service: process.env.DASHJS_SEED }];

global.dashClient = new Dash.Client({
  seeds,
});

before(async () => {
  await global.dashClient.isReady();
});
