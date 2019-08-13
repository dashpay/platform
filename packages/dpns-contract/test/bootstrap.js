const { expect, use } = require('chai');
const dirtyChai = require('dirty-chai');

use(dirtyChai);

global.expect = expect;
