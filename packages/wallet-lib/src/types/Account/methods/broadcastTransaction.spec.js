const { expect } = require('chai');
const Dashcore = require('@dashevo/dashcore-lib');
const broadcastTransaction = require('./broadcastTransaction');
const validRawTxs = require('../../../../fixtures/rawtx').valid;
const invalidRawTxs = require('../../../../fixtures/rawtx').invalid;
const expectThrowsAsync = require('../../../utils/expectThrowsAsync');

describe('Account - broadcastTransaction', () => {
  it('should throw error on missing transport', async () => {
    const expectedException1 = 'A transport layer is needed to perform a broadcast';
    const self = {
      transporter: {
        isValid: false,
      },
    };
    expectThrowsAsync(async () => await broadcastTransaction.call(self, validRawTxs.tx2to2Testnet), expectedException1);

    // return broadcastTransaction
    //   .call(self, validRawTxs.tx2to2Testnet)
    //   .then(
    //     (e) => Promise.reject(new Error('Expected method to reject.'+e)),
    //     err => expect(err).to.be.a('Error').with.property('message', expectedException1),
    //   );
  });
  it('should throw error on invalid rawtx (string)', async () => {
    const expectedException1 = 'A valid transaction object or it\'s hex representation is required';
    const self = {
      transporter: {
        isValid: true,
      },
    };

    expectThrowsAsync(async () => await broadcastTransaction.call(self, invalidRawTxs.notRelatedString), expectedException1);
  });
  it('should throw error on invalid rawtx (hex)', async () => {
    const expectedException1 = 'A valid transaction object or it\'s hex representation is required';
    const self = {
      transporter: {
        isValid: true,
      },
    };

    expectThrowsAsync(async () => await broadcastTransaction.call(self, invalidRawTxs.truncatedRawTx), expectedException1);
  });
  it('should work on valid Transaction object', async () => {
    let sendCalled = +1;
    let searchCalled = +1;
    const self = {
      transporter: {
        isValid: true,
        sendTransaction: () => sendCalled = +1,
      },
      storage: {
        searchAddress: () => { searchCalled = +1; return { found: false }; },
        searchAddressesWithTx: () => { searchCalled = +1; return { results: [] }; },
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
      transporter: {
        isValid: true,
        sendTransaction: () => sendCalled = +1,
      },
      storage: {
        searchAddress: () => { searchCalled = +1; return { found: false }; },
        searchAddressesWithTx: () => { searchCalled = +1; return { results: [] }; },
      },
    };

    return broadcastTransaction
      .call(self, validRawTxs.tx1to1Mainnet)
      .then(
        () => expect(sendCalled).to.equal(1) && expect(searchCalled).to.equal(1),
      );
  });
  it('should update affected tx', () => {
    let sendCalled = +1;
    let searchCalled = +1;
    const self = {
      transporter: {
        isValid: true,
        sendTransaction: () => sendCalled = +1,
      },
      storage: {
        searchAddress: () => { searchCalled = +1; return { found: false }; },
        searchAddressesWithTx: (affectedTxId) => { searchCalled = +1; return { results: [] }; },
      },
    };

    return broadcastTransaction
      .call(self, validRawTxs.tx1to1Mainnet)
      .then(
        () => expect(sendCalled).to.equal(1) && expect(searchCalled).to.equal(1),
      );
  });
});
