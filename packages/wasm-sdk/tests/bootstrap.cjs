// Allow self-signed certificates for local dashmate nodes
process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0';

const chai = require('chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');

chai.use(chaiAsPromised);
chai.use(dirtyChai);

const { expect } = chai;
const g = (typeof globalThis !== 'undefined') ? globalThis : global;
g.expect = expect;
