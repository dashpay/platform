const { expect } = require('chai');

const Insight = require('../../lib/insight/insight');

// Stubs
// TODO: add stubs for network
const app = { config: { livenet: false } };

// Disable no-undef rule for mocha
/* eslint-disable no-undef */
describe('Insight', () => {
  const insight = new Insight(app);

  describe('.getLastBlockHash', () => {
    it('should return block hash', () => insight.getLastBlockHash().then((lastBlockHash) => {
      expect(lastBlockHash).to.be.a('string');
    }));
  });

  describe('.getMnList', () => {
    it('should return mn list', () => insight.getMnList().then((MNList) => {
      expect(MNList).to.be.an('array');
      expect(MNList[0]).to.be.an('object');
      expect(MNList[0].vin).to.be.a('string');
      expect(MNList[0].status).to.be.a('string');
      expect(MNList[0].rank).to.be.a('number');
      expect(MNList[0].ip).to.be.a('string');
      expect(MNList[0].protocol).to.be.a('number');
      expect(MNList[0].payee).to.be.a('string');
      expect(MNList[0].activeseconds).to.be.a('number');
      expect(MNList[0].lastseen).to.be.a('number');
    }));
  });

  describe('.getAddress', () => {
    const txHash = '50622f66236671501c0e80f388d6cf1e81158de8526f4acc9db00adf3c524077';
    it('should return address', () => insight.getAddress(txHash).then((address) => {
      expect(address).to.be.a('string');
    }));
  });
});

