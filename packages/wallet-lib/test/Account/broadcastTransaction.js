const { expect } = require('chai');
const Dashcore = require('@dashevo/dashcore-lib');
const broadcastTransaction = require('../../src/Account/broadcastTransaction');
const validRawTxs = require('../fixtures/rawtx').valid;
const invalidRawTxs = require('../fixtures/rawtx').invalid;

describe('Account - broadcastTransaction', () => {
  it('should throw error on missing transport', async () => {
    const expectedException1 = 'A transport layer is needed to perform a broadcast';
    const self = {
      transport: {
        isValid: false,
      },
    };

    return broadcastTransaction
      .call(self, validRawTxs.tx2to2Testnet)
      .then(
        () => Promise.reject(new Error('Expected method to reject.')),
        err => expect(err).to.be.a('Error').with.property('message', expectedException1),
      );
  });
  it('should throw error on invalid rawtx (string)', async () => {
    const expectedException1 = 'A valid transaction object or it\'s hex representation is required';
    const self = {
      transport: {
        isValid: true,
      },
    };

    return broadcastTransaction
      .call(self, invalidRawTxs.notRelatedString)
      .then(
        () => Promise.reject(new Error('Expected method to reject.')),
        err => expect(err).to.be.a('Error').with.property('message', expectedException1),
      );
  });
  it('should throw error on invalid rawtx (hex)', async () => {
    const expectedException1 = 'A valid transaction object or it\'s hex representation is required';
    const self = {
      transport: {
        isValid: true,
      },
    };

    return broadcastTransaction
      .call(self, invalidRawTxs.truncatedRawTx)
      .then(
        () => Promise.reject(new Error('Expected method to reject.')),
        err => expect(err).to.be.a('Error').with.property('message', expectedException1),
      );
  });
  it('should work on valid Transaction object', async () => {
    let sendCalled = +1;
    let searchCalled = +1;
    const self = {
      transport: {
        isValid: true,
        sendRawTransaction: () => sendCalled = +1,
      },
      storage: {
        searchAddressWithTx: () => { searchCalled = +1; return { type: null }; },
      },
    };

    const tx = new Dashcore.Transaction(validRawTxs.tx1to1Mainnet);
    return broadcastTransaction
      .call(self, tx)
      .then(
        () => expect(sendCalled).to.equal(1) && expect(searchCalled).to.equal(1),
      );
  });
  it('should work on valid rawtx', async () => {
    let sendCalled = +1;
    let searchCalled = +1;
    const self = {
      transport: {
        isValid: true,
        sendRawTransaction: () => sendCalled = +1,
      },
      storage: {
        searchAddressWithTx: () => { searchCalled = +1; return { type: null }; },
      },
    };

    return broadcastTransaction
      .call(self, validRawTxs.tx1to1Mainnet)
      .then(
        () => expect(sendCalled).to.equal(1) && expect(searchCalled).to.equal(1),
      );
  });
});
