const chai = require('chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');

chai.use(chaiAsPromised);
chai.use(dirtyChai);

const { expect } = chai;
const g = (typeof globalThis !== 'undefined') ? globalThis : global;
g.expect = expect;

