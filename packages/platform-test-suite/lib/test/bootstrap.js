const path = require('path');
const dotenvSafe = require('dotenv-safe');
const { expect, use } = require('chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');

use(chaiAsPromised);
use(dirtyChai);

process.env.NODE_ENV = 'test';

dotenvSafe.config({
  path: path.resolve(__dirname, '..', '..', '.env'),
});

global.expect = expect;
