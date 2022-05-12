const { expect } = require('chai');
const EventEmitter = require('events');
const Dashcore = require('@dashevo/dashcore-lib');

const broadcastTransaction = require('./broadcastTransaction');
const EVENTS = require('../../../EVENTS');
const validRawTxs = require('../../../../fixtures/rawtx').valid;
const invalidRawTxs = require('../../../../fixtures/rawtx').invalid;
const MempoolPropagationTimeoutError = require('../../../errors/MempoolPropagationTimeoutError');
const ChainStore = require('../../ChainStore/ChainStore');
const { PrivateKey } = Dashcore;

describe('Account - broadcastTransaction', function suite() {
  this.timeout(10000);
  let utxos;
  let address;
  let keysToSign;
  let oneToOneTx;
  let fee;

  const chainStore = new ChainStore('testnet');
  chainStore.state.fees.minRelay = 888;
  chainStore.importAddress('yTBXsrcGw74yMUsK34fBKAWJx3RNCq97Aq');
  const storage = {
    getChainStore:()=> chainStore
  }

  let self;
  let sendCalled = 0;

  beforeEach(function () {
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

    sendCalled = 0;
    self = new EventEmitter();
    self.removeListener = this.sinonSandbox.spy();
    self.transport = {
      sendTransaction: (txHex) => {
        const transaction = new Dashcore.Transaction(txHex)
        self.emit(EVENTS.FETCHED_CONFIRMED_TRANSACTION, {
          payload: {
            transaction
          }
        });

        sendCalled += 1;
        return transaction.hash
      },
    };
    self.network = 'testnet';
    self.storage = storage;
  });


  it('should throw error on missing transport', async function () {
    const expectedException1 = 'A transport layer is needed to perform a broadcast';
    self.transport = null;

    await expect(broadcastTransaction.call(self, validRawTxs.tx2to2Testnet))
      .to.be.rejectedWith(expectedException1);

    expect(self.removeListener).to.have.been.calledOnceWith(EVENTS.FETCHED_CONFIRMED_TRANSACTION);
  });

  it('should throw error on invalid rawtx (string)', async () => {
    const expectedException1 = 'A valid transaction object or it\'s hex representation is required';

    await expect(broadcastTransaction.call(self, invalidRawTxs.notRelatedString))
      .to.be.rejectedWith(expectedException1);
    expect(self.removeListener).to.have.been.calledOnceWith(EVENTS.FETCHED_CONFIRMED_TRANSACTION);
  });

  it('should throw error on invalid rawtx (hex)', async () => {
    const expectedException1 = 'A valid transaction object or it\'s hex representation is required';

    await expect(broadcastTransaction.call(self, invalidRawTxs.truncatedRawTx))
      .to.be.rejectedWith(expectedException1);
    expect(self.removeListener).to.have.been.calledOnceWith(EVENTS.FETCHED_CONFIRMED_TRANSACTION);
  });

  it('should work on valid Transaction object', async () => {
    return broadcastTransaction
      .call(self, oneToOneTx)
      .then(
        () => expect(sendCalled).to.equal(1)
      );
  });

  it('should throw error on fee not met', async function () {
    const expectedException1 = 'Expected minimum fee for transaction 149. Current: 0';

    oneToOneTx.fee(0);

    await expect(broadcastTransaction.call(self, oneToOneTx))
      .to.be.rejectedWith(expectedException1);
    expect(self.removeListener).to.have.been.calledOnceWith(EVENTS.FETCHED_CONFIRMED_TRANSACTION);
  });

  it('should broadcast when force and fee not met', function () {
    oneToOneTx.fee(0);

    return broadcastTransaction
      .call(self, oneToOneTx, { skipFeeValidation: true })
      .then(
        () => expect(sendCalled).to.equal(1)
      );
  });

  it('should throw mempool propagation timeout error', async function () {
    self.transport.sendTransaction = () => new Promise(() => {})

    await expect(broadcastTransaction.call(self, oneToOneTx, {
      mempoolPropagationTimeout: 1
    })).to.be.rejectedWith(MempoolPropagationTimeoutError);

    expect(self.removeListener).to.have.been.calledOnceWith(EVENTS.FETCHED_CONFIRMED_TRANSACTION);
  });
});
