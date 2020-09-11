const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const EventEmitter = require('events');
const { WALLET_TYPES } = require('../../../../src/CONSTANTS');
const importTransactions = require('../../../../src/types/Account/methods/importTransactions');
const getAddress = require('../../../../src/types/Account/methods/getAddress');
const generateAddress = require('../../../../src/types/Account/methods/generateAddress');
const importBlockHeader = require('../../../../src/types/Account/methods/importBlockHeader');
const _initializeAccount = require('../../../../src/types/Account/_initializeAccount');

const {
  HDPrivateKey,
  Transaction,
  MerkleBlock,
} = require('@dashevo/dashcore-lib')

const TransactionSyncStreamWorker = require('../../../../src/plugins/Workers/TransactionSyncStreamWorker/TransactionSyncStreamWorker');
const Storage = require('../../../../src/types/Storage/Storage');
const KeyChain = require('../../../../src/types/KeyChain/KeyChain');

const TxStreamDataResponseMock = require('../../../../src/test/mocks/TxStreamDataResponseMock');
const TxStreamMock = require('../../../../src/test/mocks/TxStreamMock');

chai.use(chaiAsPromised);
const { expect } = chai;

function wait(ms) {
  return new Promise((res) => setTimeout(res, ms));
}

const BIP44PATH = `m/44'/1'/0'`

describe('TransactionSyncStreamWorker', function suite() {
  this.timeout(60000);
  let worker;
  let mockParentEmitter;
  let storage;
  let walletId;
  let walletType;
  let accountMock;
  let txStreamMock;
  let address;
  let network;
  let addressAtIndex19;
  let keyChain;
  let testHDKey;
  let merkleBlockBuffer;
  let merkleBlockMock;

  beforeEach(function beforeEach() {
    network = 'testnet';
    testHDKey = "xprv9s21ZrQH143K4PgfRZPuYjYUWRZkGfEPuWTEUESMoEZLC274ntC4G49qxgZJEPgmujsmY52eVggtwZgJPrWTMXmbYgqDVySWg46XzbGXrSZ";
    merkleBlockBuffer = Buffer.from([0,0,0,32,61,11,102,108,38,155,164,49,91,246,141,178,126,155,13,118,248,83,250,15,206,21,102,65,104,183,243,167,235,167,60,113,140,110,120,87,208,191,240,19,212,100,228,121,192,125,143,44,226,9,95,98,51,25,139,172,175,27,205,201,158,85,37,8,72,52,36,95,255,255,127,32,2,0,0,0,1,0,0,0,1,140,110,120,87,208,191,240,19,212,100,228,121,192,125,143,44,226,9,95,98,51,25,139,172,175,27,205,201,158,85,37,8,1,1]);
    merkleBlockMock = new MerkleBlock(merkleBlockBuffer);

    txStreamMock = new TxStreamMock();

    walletType = WALLET_TYPES.HDWALLET;

    storage = new Storage();
    keyChain = new KeyChain({ HDPrivateKey: new HDPrivateKey(testHDKey) });

    testHDKey = new HDPrivateKey(testHDKey).toString();
    mockParentEmitter = Object.create(EventEmitter.prototype);
    storage.createWallet();
    walletId = Object.keys(storage.store.wallets)[0];

    accountMock = new EventEmitter();
    Object.assign(accountMock, {
      transport: {
        getBestBlockHeight: this.sinonSandbox.stub().returns(42),
        subscribeToTransactionsWithProofs: this.sinonSandbox.stub().returns(txStreamMock),
      },
      injectDefaultPlugins: true,
      storage,
      keyChain,
      store: storage.store,
      walletId,
      walletType,
      index: 0,
      network,
      BIP44PATH,
      getAddress,
      importTransactions,
      generateAddress,
      importBlockHeader,
      injectPlugin: this.sinonSandbox.stub(),
      plugins: {
        watchers: [],
      },
      state: {}
    });

    _initializeAccount(accountMock, []);

    // That sets the last synced block
    storage.store.wallets[walletId].accounts[BIP44PATH] = {};

    worker = new TransactionSyncStreamWorker();

    Object.assign(worker, accountMock);

    worker.setLastSyncedBlockHeight(1);
    worker.parentEvents = mockParentEmitter;

    address = accountMock.getAddress(0).address;
    addressAtIndex19 = accountMock.getAddress(19).address;
  });

  afterEach(() => {
    worker.stopWorker();
  })

  describe("#onStart", () => {
    it('should sync historical data from the last saved block', async function () {
      const lastSavedBlockHeight = 40;
      const bestBlockHeight = 42;

      worker.setLastSyncedBlockHeight(lastSavedBlockHeight);
      const transactionsSent = [];

      accountMock.transport.getBestBlockHeight
        .returns(bestBlockHeight);

      setTimeout(async () => {
        try {
          expect(worker.stream).is.not.null;

          for (let i = lastSavedBlockHeight; i <= bestBlockHeight; i++) {
            const transaction = new Transaction().to(address, i);

            transactionsSent.push(transaction);
            txStreamMock.emit(TxStreamMock.EVENTS.data, new TxStreamDataResponseMock({
              rawTransactions: [transaction.toBuffer()]
            }));
            await wait(10);
          }

          txStreamMock.emit(TxStreamMock.EVENTS.end);
        } catch (e) {
          console.error(e);
          txStreamMock.emit(TxStreamMock.EVENTS.error, e);
        }
      }, 10);

      await worker.onStart();

      const transactionsInStorage = Object
          .values(storage.getStore().transactions)
          .map((t) => t.toJSON());

      const expectedTransactions = transactionsSent
          .map((t) => t.toJSON());

      expect(worker.stream).to.be.null;
      expect(transactionsInStorage.length).to.be.equal(3);
      expect(transactionsInStorage).to.have.deep.members(expectedTransactions);
    });

    it('should reconnect to the historical stream when gap limit is filled', async function () {
      const lastSavedBlockHeight = 40;
      const bestBlockHeight = 42;

      worker.setLastSyncedBlockHeight(lastSavedBlockHeight);
      const transactionsSent = [];

      accountMock.transport.getBestBlockHeight
        .returns(bestBlockHeight);

      setTimeout(async () => {
        try {
          expect(worker.stream).is.not.null;

          let transaction = new Transaction().to(addressAtIndex19, 10000);

          transactionsSent.push(transaction);
          txStreamMock.emit(TxStreamMock.EVENTS.data, new TxStreamDataResponseMock({
            rawTransactions: [transaction.toBuffer()]
          }));

          await wait(10);

          merkleBlockMock.hashes[0] =  Buffer.from(transaction.hash, 'hex').reverse().toString('hex');
          txStreamMock.emit(TxStreamMock.EVENTS.data, new TxStreamDataResponseMock({
            rawMerkleBlock: merkleBlockMock.toBuffer()
          }));

          await wait(10);

          transaction = new Transaction().to(accountMock.getAddress(10).address, 10000);

          transactionsSent.push(transaction);
          txStreamMock.emit(TxStreamMock.EVENTS.data, new TxStreamDataResponseMock({
            rawTransactions: [transaction.toBuffer()]
          }));

          await wait(10);

          txStreamMock.emit(TxStreamMock.EVENTS.end);
        } catch (e) {
          console.error(e);
          txStreamMock.emit(TxStreamMock.EVENTS.error, e);
        }
      }, 10);

      await worker.onStart();

      const transactionsInStorage = Object
          .values(storage.getStore().transactions)
          .map((t) => t.toJSON());

      const expectedTransactions = transactionsSent
          .map((t) => t.toJSON());


      const addressesInStorage = storage.store.wallets[walletId].addresses.external;
      // We send transaction to index 19, so wallet should generate additional 20 addresses to keep the gap between
      // the last used address
      expect(Object.keys(addressesInStorage).length).to.be.equal(40);
      // It should reconnect after the gap limit is reached
      expect(accountMock.transport.subscribeToTransactionsWithProofs.callCount).to.be.equal(3);
      // 20 external and 20 internal
      expect(accountMock.transport.subscribeToTransactionsWithProofs.firstCall.args[0].length).to.be.equal(40);
      expect(accountMock.transport.subscribeToTransactionsWithProofs.firstCall.args[1]).to.be.deep.equal({ fromBlockHeight: 40, count: 2});
      // 20 more of each type, since the last address is used, but the height is the same, since Merkle Block not received yet
      expect(accountMock.transport.subscribeToTransactionsWithProofs.secondCall.args[0].length).to.be.equal(80);
      expect(accountMock.transport.subscribeToTransactionsWithProofs.secondCall.args[1]).to.be.deep.equal({ fromBlockHeight: 40, count: 2});
      // Block received
      expect(accountMock.transport.subscribeToTransactionsWithProofs.thirdCall.args[0].length).to.be.equal(80);
      expect(accountMock.transport.subscribeToTransactionsWithProofs.thirdCall.args[1]).to.be.deep.equal({ fromBlockHash: '5e55b2ca5472098231965e87a80b35750554ad08d5a1357800b7cd0dfa153646', count: 2});

      expect(worker.stream).to.be.null;
      expect(transactionsInStorage.length).to.be.equal(2);
      expect(transactionsInStorage).to.have.deep.members(expectedTransactions);
    });

    it('should reconnect to the historical stream if stream is closed due to operational GRPC error', async function () {
      const lastSavedBlockHeight = 40;
      const bestBlockHeight = 42;

      worker.setLastSyncedBlockHeight(lastSavedBlockHeight);
      const transactionsSent = [];

      accountMock.transport.getBestBlockHeight
          .returns(bestBlockHeight);

      setTimeout(async () => {
        expect(worker.stream).is.not.null;

        const err = new Error('Some error');
        err.code = 4;
        txStreamMock.emit(TxStreamMock.EVENTS.error, err);

        await wait(10);

        txStreamMock.emit(TxStreamMock.EVENTS.end);
      }, 10);

      await worker.onStart();

      const addressesInStorage = storage.store.wallets[walletId].addresses.external;

      expect(Object.keys(addressesInStorage).length).to.be.equal(20);
      // It should reconnect after because of the operational error
      expect(accountMock.transport.subscribeToTransactionsWithProofs.callCount).to.be.equal(2);
      // 20 external and 20 internal
      expect(accountMock.transport.subscribeToTransactionsWithProofs.firstCall.args[0].length).to.be.equal(40);
      expect(accountMock.transport.subscribeToTransactionsWithProofs.firstCall.args[1]).to.be.deep.equal({ fromBlockHeight: 40, count: 2});

      expect(accountMock.transport.subscribeToTransactionsWithProofs.secondCall.args[0].length).to.be.equal(40);
      expect(accountMock.transport.subscribeToTransactionsWithProofs.secondCall.args[1]).to.be.deep.equal({ fromBlockHeight: 40, count: 2});

      expect(worker.stream).to.be.null;
    });

    it('should not reconnect to the historical stream if stream in case of any other error', async function () {
      const lastSavedBlockHeight = 40;
      const bestBlockHeight = 42;

      worker.setLastSyncedBlockHeight(lastSavedBlockHeight);
      const transactionsSent = [];

      accountMock.transport.getBestBlockHeight
          .returns(bestBlockHeight);

      setTimeout(async () => {
        expect(worker.stream).is.not.null;

        txStreamMock.emit(TxStreamMock.EVENTS.error, new Error('Some random error'));
      }, 10);

      await expect(worker.onStart()).to.be.rejectedWith('Some random error');

      const addressesInStorage = storage.store.wallets[walletId].addresses.external;

      expect(Object.keys(addressesInStorage).length).to.be.equal(20);
      // Shouldn't try to reconnect
      expect(accountMock.transport.subscribeToTransactionsWithProofs.callCount).to.be.equal(1);
      // 20 external and 20 internal
      expect(accountMock.transport.subscribeToTransactionsWithProofs.firstCall.args[0].length).to.be.equal(40);
      expect(accountMock.transport.subscribeToTransactionsWithProofs.firstCall.args[1]).to.be.deep.equal({ fromBlockHeight: 40, count: 2});

      expect(worker.stream).to.be.null;
    });
  });

  describe("#execute", () => {
    it('should sync incoming transactions and save it to the storage', async function () {
      const lastSavedBlockHeight = 40;
      const bestBlockHeight = 42;

      worker.setLastSyncedBlockHeight(lastSavedBlockHeight);
      const transactionsSent = [];

      accountMock.transport.getBestBlockHeight
          .returns(bestBlockHeight);

      worker.execute();

      await wait(10);

      try {
        for (let i = lastSavedBlockHeight; i <= bestBlockHeight; i++) {
          const transaction = new Transaction().to(address, i);

          transactionsSent.push(transaction);
          txStreamMock.emit(TxStreamMock.EVENTS.data, new TxStreamDataResponseMock({
            rawTransactions: [transaction.toBuffer()]
          }));
          await wait(10);
        }

        txStreamMock.emit(TxStreamMock.EVENTS.end);
      } catch (e) {
        console.error(e);
        txStreamMock.emit(TxStreamMock.EVENTS.error, e);
      }

      await worker.onStop();

      const transactionsInStorage = Object
          .values(storage.getStore().transactions)
          .map((t) => t.toJSON());

      const expectedTransactions = transactionsSent
          .map((t) => t.toJSON());

      expect(worker.stream).to.be.null;
      expect(transactionsInStorage.length).to.be.equal(3);
      expect(transactionsInStorage).to.have.deep.members(expectedTransactions);
    })
    it('should reconnect to the incoming stream when gap limit is filled', async function () {
      const lastSavedBlockHeight = 40;
      const bestBlockHeight = 42;

      worker.setLastSyncedBlockHeight(lastSavedBlockHeight);
      const transactionsSent = [];

      accountMock.transport.getBestBlockHeight
          .returns(bestBlockHeight);

      worker.execute();

      await wait(10);

      try {
        let transaction = new Transaction().to(addressAtIndex19, 10000);

        transactionsSent.push(transaction);
        txStreamMock.emit(TxStreamMock.EVENTS.data, new TxStreamDataResponseMock({
          rawTransactions: [transaction.toBuffer()]
        }));

        await wait(10);

        merkleBlockMock.hashes[0] =  Buffer.from(transaction.hash, 'hex').reverse().toString('hex');
        txStreamMock.emit(TxStreamMock.EVENTS.data, new TxStreamDataResponseMock({
          rawMerkleBlock: merkleBlockMock.toBuffer()
        }));

        await wait(10);

        transaction = transaction = new Transaction().to(accountMock.getAddress(10).address, 10000);

        transactionsSent.push(transaction);
        txStreamMock.emit(TxStreamMock.EVENTS.data, new TxStreamDataResponseMock({
          rawTransactions: [transaction.toBuffer()]
        }));

        await wait(10);

        txStreamMock.emit(TxStreamMock.EVENTS.end);
      } catch (e) {
        console.error(e);
        txStreamMock.emit(TxStreamMock.EVENTS.error, e);
      }

      await worker.onStop();

      const transactionsInStorage = Object
          .values(storage.getStore().transactions)
          .map((t) => t.toJSON());

      const expectedTransactions = transactionsSent
          .map((t) => t.toJSON());

      const addressesInStorage = storage.store.wallets[walletId].addresses.external;
      // We send transaction to index 19, so wallet should generate additional 20 addresses to keep the gap between
      // the last used address
      expect(Object.keys(addressesInStorage).length).to.be.equal(40);
      // It should reconnect after the gap limit is reached
      expect(accountMock.transport.subscribeToTransactionsWithProofs.callCount).to.be.equal(3);
      // 20 external and 20 internal
      expect(accountMock.transport.subscribeToTransactionsWithProofs.firstCall.args[0].length).to.be.equal(40);
      expect(accountMock.transport.subscribeToTransactionsWithProofs.firstCall.args[1]).to.be.deep.equal({ fromBlockHeight: 40, count: 0});
      // 20 more of each type, since the last address is used, but the height is the same, since Merkle Block not received yet
      expect(accountMock.transport.subscribeToTransactionsWithProofs.secondCall.args[0].length).to.be.equal(80);
      expect(accountMock.transport.subscribeToTransactionsWithProofs.secondCall.args[1]).to.be.deep.equal({ fromBlockHeight: 40, count: 0});
      // Block received
      expect(accountMock.transport.subscribeToTransactionsWithProofs.thirdCall.args[0].length).to.be.equal(80);
      expect(accountMock.transport.subscribeToTransactionsWithProofs.thirdCall.args[1]).to.be.deep.equal({ fromBlockHash: '5e55b2ca5472098231965e87a80b35750554ad08d5a1357800b7cd0dfa153646', count: 0});

      expect(worker.stream).to.be.null;
      expect(transactionsInStorage.length).to.be.equal(2);
      expect(transactionsInStorage).to.have.deep.members(expectedTransactions);
    });

    it('should reconnect to the incoming stream if stream is closed due to operational GRPC error', async function () {
      const lastSavedBlockHeight = 40;
      const bestBlockHeight = 42;

      worker.setLastSyncedBlockHeight(lastSavedBlockHeight);

      accountMock.transport.getBestBlockHeight
          .returns(bestBlockHeight);

      worker.execute();

      await wait(10);

      const err = new Error('Some error');
      err.code = 4;
      txStreamMock.emit(TxStreamMock.EVENTS.error, err);

      await wait(10);

      txStreamMock.emit(TxStreamMock.EVENTS.end);

      await worker.onStop();

      const addressesInStorage = storage.store.wallets[walletId].addresses.external;

      expect(Object.keys(addressesInStorage).length).to.be.equal(20);
      // It should reconnect after the gap limit is reached
      expect(accountMock.transport.subscribeToTransactionsWithProofs.callCount).to.be.equal(2);
      // 20 external and 20 internal
      expect(accountMock.transport.subscribeToTransactionsWithProofs.firstCall.args[0].length).to.be.equal(40);
      expect(accountMock.transport.subscribeToTransactionsWithProofs.firstCall.args[1]).to.be.deep.equal({ fromBlockHeight: 40, count: 0});

      expect(accountMock.transport.subscribeToTransactionsWithProofs.secondCall.args[0].length).to.be.equal(40);
      expect(accountMock.transport.subscribeToTransactionsWithProofs.secondCall.args[1]).to.be.deep.equal({ fromBlockHeight: 40, count: 0});

      expect(worker.stream).to.be.null;
    });
    it('should reconnect to the server closes the stream without any errors', async function () {
      const lastSavedBlockHeight = 40;
      const bestBlockHeight = 42;

      worker.setLastSyncedBlockHeight(lastSavedBlockHeight);

      accountMock.transport.getBestBlockHeight
          .returns(bestBlockHeight);

      worker.execute();

      await wait(10);

      txStreamMock.emit(TxStreamMock.EVENTS.end);

      await wait(10);

      await worker.onStop();

      const addressesInStorage = storage.store.wallets[walletId].addresses.external;

      expect(Object.keys(addressesInStorage).length).to.be.equal(20);
      // It should reconnect if the server closes the stream
      expect(accountMock.transport.subscribeToTransactionsWithProofs.callCount).to.be.equal(2);
      // 20 external and 20 internal
      expect(accountMock.transport.subscribeToTransactionsWithProofs.firstCall.args[0].length).to.be.equal(40);
      expect(accountMock.transport.subscribeToTransactionsWithProofs.firstCall.args[1]).to.be.deep.equal({ fromBlockHeight: 40, count: 0});

      expect(accountMock.transport.subscribeToTransactionsWithProofs.secondCall.args[0].length).to.be.equal(40);
      expect(accountMock.transport.subscribeToTransactionsWithProofs.secondCall.args[1]).to.be.deep.equal({ fromBlockHeight: 40, count: 0});

      expect(worker.stream).to.be.null;
    });

    it('should not reconnect to the incoming stream if stream in case of any other error', async function () {
      const lastSavedBlockHeight = 40;
      const bestBlockHeight = 42;

      worker.setLastSyncedBlockHeight(lastSavedBlockHeight);

      accountMock.transport.getBestBlockHeight
          .returns(bestBlockHeight);

      worker.execute();

      await wait(10);

      txStreamMock.emit(TxStreamMock.EVENTS.error, new Error('Some random error'));

      await wait(10);

      await worker.onStop();

      await expect(worker.incomingSyncPromise).to.be.rejectedWith('Some random error');

      const addressesInStorage = storage.store.wallets[walletId].addresses.external;
      expect(Object.keys(addressesInStorage).length).to.be.equal(20);

      // Shouldn't try to reconnect
      expect(accountMock.transport.subscribeToTransactionsWithProofs.callCount).to.be.equal(1);
      // 20 external and 20 internal
      expect(accountMock.transport.subscribeToTransactionsWithProofs.firstCall.args[0].length).to.be.equal(40);
      expect(accountMock.transport.subscribeToTransactionsWithProofs.firstCall.args[1]).to.be.deep.equal({ fromBlockHeight: 40, count: 0});

      expect(worker.stream).to.be.null;
    });
  });
});
