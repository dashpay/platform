const { expect } = require('chai');
const Dashcore = require('@dashevo/dashcore-lib');
const broadcastTransaction = require('./broadcastTransaction');
const validRawTxs = require('../../../../fixtures/rawtx').valid;
const invalidRawTxs = require('../../../../fixtures/rawtx').invalid;
const expectThrowsAsync = require('../../../utils/expectThrowsAsync');

const { PrivateKey } = Dashcore;

describe('Account - broadcastTransaction', function suite() {
  this.timeout(10000);
  let utxos;
  let address;
  let keysToSign;
  let oneToOneTx;
  let fee;
  const storage = {
    getStore: ()=>({
      chains:{
          "testnet": { fees: { minRelay: 888 }}
      }
    })
  }
  beforeEach(() => {
    utxos = [
      {
        address: 'yj8sq7ogzz6JtaxpBQm5Hg9YaB5cKExn5T',
        txid: 'bfec828ed8ed562f53921e9580e847670044e870dda0e67b8f8d0c8d77962f7f',
        vout: 1,
        scriptPubKey: '76a914fa4b2bb85ad9b4075addb6d0eb50fa8b60c746c588ac',
        amount: 138.7944
      }
    ];
    fee = 680;
    address = 'yTBXsrcGw74yMUsK34fBKAWJx3RNCq97Aq';
    keysToSign = [
        new PrivateKey('26d6b24119d1a71de6372ea2d3dc22a014d37e4828b43db6936cb41ea461cce8')
    ];
    oneToOneTx = new Dashcore.Transaction()
        .from(utxos)
        .to(address, 138)
        .fee(fee);
    oneToOneTx.sign(keysToSign);
  });

  it('should throw error on missing transport', async () => {
    const expectedException1 = 'A transport layer is needed to perform a broadcast';
    const self = {
      transport: null,
      storage,
      network: 'testnet'
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
      transport: { },
      storage,
      network: 'testnet'
    };

    expectThrowsAsync(async () => await broadcastTransaction.call(self, invalidRawTxs.notRelatedString), expectedException1);
  });
  it('should throw error on invalid rawtx (hex)', async () => {
    const expectedException1 = 'A valid transaction object or it\'s hex representation is required';
    const self = {
      transport: { },
      storage,
      network: 'testnet'
    };

    expectThrowsAsync(async () => await broadcastTransaction.call(self, invalidRawTxs.truncatedRawTx), expectedException1);
  });
  it('should work on valid Transaction object', async () => {
    let sendCalled = +1;
    let searchCalled = +1;
    const self = {
      transport: {
        sendTransaction: () => sendCalled = +1,
      },
      network: 'testnet',
      storage: {
        getStore: storage.getStore,
        searchAddress: () => { searchCalled = +1; return { found: false }; },
        searchAddressesWithTx: () => { searchCalled = +1; return { results: [] }; },
      },
    };

    const tx = oneToOneTx;
    return broadcastTransaction
      .call(self, tx)
      .then(
        () => expect(sendCalled).to.equal(1) && expect(searchCalled).to.equal(1),
      );
  });
  it('should update affected tx', () => {
    let sendCalled = +1;
    let searchCalled = +1;
    const self = {
      transport: {
        sendTransaction: () => sendCalled = +1,
      },
      network: 'testnet',
      storage: {
        getStore: storage.getStore,
        searchAddress: () => { searchCalled = +1; return { found: false }; },
        searchAddressesWithTx: (affectedTxId) => { searchCalled = +1; return { results: [] }; },
      },
    };

    return broadcastTransaction
      .call(self, oneToOneTx)
      .then(
        () => expect(sendCalled).to.equal(1) && expect(searchCalled).to.equal(1),
      );
  });
  it('should throw error on fee not met', function () {
    const expectedException1 = 'Expected minimum fee for transaction 149. Current: 0\n';

    let sendCalled = +1;
    let searchCalled = +1;
    const self = {
      transport: {
        sendTransaction: () => sendCalled = +1,
      },
      network: 'testnet',
      storage: {
        getStore: storage.getStore,
        searchAddress: () => { searchCalled = +1; return { found: false }; },
        searchAddressesWithTx: () => { searchCalled = +1; return { results: [] }; },
      },
    };

    const tx = oneToOneTx;
    tx.fee(0);
    expectThrowsAsync(async () => await broadcastTransaction.call(self, invalidRawTxs.truncatedRawTx), expectedException1);
  });
  it('should broadcast when force and fee not met', function () {
    let sendCalled = +1;
    let searchCalled = +1;
    const self = {
      transport: {
        sendTransaction: () => sendCalled = +1,
      },
      network: 'testnet',
      storage: {
        getStore: storage.getStore,
        searchAddress: () => { searchCalled = +1; return { found: false }; },
        searchAddressesWithTx: () => { searchCalled = +1; return { results: [] }; },
      },
    };

    const tx = oneToOneTx;
    tx.fee(0);

    return broadcastTransaction
        .call(self, tx, {skipFeeValidation: true})
        .then(
            () => expect(sendCalled).to.equal(1) && expect(searchCalled).to.equal(1),
        );
  });
});
