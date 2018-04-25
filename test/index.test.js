const Blockchain = require('../lib/spvchain');
const chainManager = require('./chainmanager');

let chain = null;
let headers = [];
require('should');

describe('SPV-DASH (forks & re-orgs)', () => {
  before(() => {
    headers = chainManager.fetchHeaders();
    chain = new Blockchain('testnet');
  });

  it('should get 25 testnet headers', () => {
    headers.length.should.equal(25);
  });

  it('should contain 1 fork when chain is initialised with genesis block', () => {
    chain.forkedChains.length.should.equal(1);
  });

  it('should contain genesis hash', () => {
    chain.getTipHash().should.equal('00000bafbc94add76cb75e2ec92894837288a481e5c005f6563d91623bf8bc2c');
  });

  it('should still contain a fork of 1 when first header is added', () => {
    chain.addHeader(headers[0]);
    chain.forkedChains.length.should.equal(1);
  });

  it('should contain correct tip and height when remaining headers [1..24] is added', () => {
    headers.shift();
    chain.addHeaders(headers);
    chain.getChainHeight().should.equal(25);
  });
});

describe('SPV-DASH (merkle proofs)', () => {
  // Tests included in dapi-sdk
  // possibly add further tests here
});

