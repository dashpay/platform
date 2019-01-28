const { expect, use } = require('chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');

use(chaiAsPromised);
use(dirtyChai);

global.expect = expect;
