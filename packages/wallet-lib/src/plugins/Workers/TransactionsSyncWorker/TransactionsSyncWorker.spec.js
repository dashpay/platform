const { expect } = require('chai');
const EventEmitter = require('events');
const TransactionsSyncWorker = require('./TransactionsSyncWorker');
const TransactionsReader = require('./TransactionsReader');
const { waitOneTick } = require('../../../test/utils');

describe('TransactionsSyncWorker', () => {
  let transactionsSyncWorker;

  const CHAIN_HEIGHT = 1000;
  const ADDRESSES_KEYCHAIN_1 = ['XqbqiYFC45SD1E3V1yRHmVLenZKoF4dwfH', 'Xmz1nb4ikHw374SbEHt81AnRh6kV2vz6LM', 'XwbDo3myVfbRbAQ4fHWrjsFKEBqiK4Sz7P'];
  const ADDRESSES_KEYCHAIN_2 = ['XtmEv7XAHJXHBommfR7WhRU28E2oMgUxgJ', 'Xs1GTJEENg4SdJaEoGmgJRiC1L1QBzgfS6'];
  const ADDRESSES_KEYCHAIN_3 = ['XbrMntvw3KR1wEmBM9xMbB9hiNi7NNrwEG'];

  const ADDRESSES = [
    ADDRESSES_KEYCHAIN_1,
    ADDRESSES_KEYCHAIN_2,
    ADDRESSES_KEYCHAIN_3,
  ];

  const createTransactionsSyncWorker = (sinon) => {
    const worker = new TransactionsSyncWorker({
      executeOnStart: false,
    });

    worker.keyChainStore = {
      getKeyChains: () => ADDRESSES.map((addresses) => ({
        getWatchedAddresses: () => addresses,
      })),
    };

    const transactionsReader = new EventEmitter();
    transactionsReader.startHistoricalSync = sinon.spy();
    transactionsReader.startContinuousSync = sinon.spy();
    sinon.spy(transactionsReader, 'on');
    sinon.spy(transactionsReader, 'once');
    sinon.spy(transactionsReader, 'removeListener');

    worker.transactionsReader = transactionsReader;

    worker.storage = {
      application: {},
      scheduleStateSave: sinon.spy(),
      saveState: sinon.spy(),
      getDefaultChainStore() {
        if (!this.defaultChainStore) {
          this.defaultChainStore = {
            state: {
              chainHeight: CHAIN_HEIGHT,
              lastSyncedBlockHeight: -1,
            },
            clearHeadersMetadata: sinon.spy(),
            // updateLastSyncedBlockHeight: sinon.spy(),
            // updateChainHeight: sinon.spy(),
            // setBlockHeaders: sinon.spy(),
            // updateHeadersMetadata: sinon.spy(),
          };
        }
        return this.defaultChainStore;
      },
    };

    return worker;
  };

  describe('#getStartBlockHeight', () => {
    beforeEach(function beforeEach() {
      transactionsSyncWorker = createTransactionsSyncWorker(this.sinon);
    });

    it('should return block 1 by default', () => {
      const startBlockHeight = transactionsSyncWorker.getStartBlockHeight();
      expect(startBlockHeight).to.equal(1);
    });

    it('should return last synced block height if present', () => {
      const { storage } = transactionsSyncWorker;
      storage.getDefaultChainStore().state.lastSyncedBlockHeight = 1200;

      const startBlockHeight = transactionsSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1200);
    });

    it('should return chain height in case `skipSynchronization` option is present', () => {
      /**
       * Mock options
       */
      const { storage } = transactionsSyncWorker;
      storage.getDefaultChainStore().state.chainHeight = CHAIN_HEIGHT;

      storage.application.syncOptions = {
        skipSynchronization: true,
      };

      const startBlockHeight = transactionsSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(CHAIN_HEIGHT);
    });

    it('should return `skipSynchronizationBeforeHeight` value', () => {
      const { storage } = transactionsSyncWorker;
      storage.application.syncOptions = {
        skipSynchronizationBeforeHeight: 1300,
      };

      const startBlockHeight = transactionsSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1300);
    });

    it('should return `skipSynchronizationBeforeHeight` value if it\'s greater than last synced header height', () => {
      const { storage } = transactionsSyncWorker;
      storage.getDefaultChainStore().state.lastSyncedHeaderHeight = 1200;
      storage.application.syncOptions = {
        skipSynchronizationBeforeHeight: 1300,
      };

      const startBlockHeight = transactionsSyncWorker.getStartBlockHeight();

      expect(startBlockHeight).to.equal(1300);
    });
  });

  describe('#onStart', () => {
    beforeEach(function beforeEach() {
      transactionsSyncWorker = createTransactionsSyncWorker(this.sinon);
    });

    it('should kickstart reading of historical headers', async () => {
      const startPromise = transactionsSyncWorker.onStart();
      await waitOneTick();

      const { transactionsReader } = transactionsSyncWorker;

      expect(transactionsSyncWorker.syncState).to
        .equal(TransactionsSyncWorker.STATES.HISTORICAL_SYNC);
      expect(transactionsReader.on).to
        .have.been.calledWith(
          TransactionsReader.EVENTS.HISTORICAL_TRANSACTIONS,
          transactionsSyncWorker.historicalSyncHandler,
        );

      expect(transactionsReader.on).to
        .have.been.calledWith(
          TransactionsReader.EVENTS.ERROR,
          transactionsSyncWorker.transactionsReaderErrorHandler,
        );
      expect(transactionsReader.once).to
        .have.been.calledWith(
          TransactionsReader.EVENTS.HISTORICAL_DATA_OBTAINED,
          transactionsSyncWorker.historicalDataObtainedHandler,
        );
      expect(transactionsReader.once).to
        .have.been.calledWith(
          TransactionsReader.EVENTS.STOPPED,
          transactionsSyncWorker.transactionsReaderStoppedHandler,
        );
      expect(transactionsReader.startHistoricalSync)
        .to.have.been.calledOnceWith(
          1,
          CHAIN_HEIGHT,
          ADDRESSES.reduce((acc, addresses) => acc.concat(addresses), []),
        );
      transactionsReader.emit(TransactionsReader.EVENTS.HISTORICAL_DATA_OBTAINED);
      await startPromise;
    });

    it('should skip historical sync in case startBlockHeight is equal to chain height', async () => {
      const { storage, transactionsReader } = transactionsSyncWorker;

      // Invalid chain height
      storage.getDefaultChainStore().state.lastSyncedBlockHeight = CHAIN_HEIGHT;
      await transactionsSyncWorker.onStart();

      const chainStore = storage.getDefaultChainStore();
      expect(chainStore.clearHeadersMetadata).to.have.been.calledOnce();
      expect(transactionsReader.startHistoricalSync).to.have.not.been.called();
    });

    it('should prepare for continuous sync after historical data is obtained', async function test() {
      const { transactionsReader } = transactionsSyncWorker;

      transactionsSyncWorker.updateProgress = this.sinon.spy();

      const startPromise = transactionsSyncWorker.onStart();
      transactionsReader.emit(TransactionsReader.EVENTS.HISTORICAL_DATA_OBTAINED);
      await startPromise;

      expect(transactionsSyncWorker.syncState).to
        .equal(TransactionsSyncWorker.STATES.IDLE);
      expect(transactionsReader.removeListener)
        .to.have.been.calledWith(
          TransactionsReader.EVENTS.HISTORICAL_TRANSACTIONS,
          transactionsSyncWorker.historicalSyncHandler,
        );
      expect(transactionsReader.removeListener)
        .to.have.been.calledWith(
          TransactionsReader.EVENTS.ERROR,
          transactionsSyncWorker.transactionsReaderErrorHandler,
        );
      expect(transactionsSyncWorker.syncCheckpoint).to.equal(CHAIN_HEIGHT);
      expect(transactionsSyncWorker.updateProgress).to.have.been.calledOnce();
      expect(transactionsSyncWorker.storage.saveState).to.have.been.calledOnce();

      const chainStore = transactionsSyncWorker.storage.getDefaultChainStore();
      expect(chainStore.clearHeadersMetadata).to.have.been.calledOnce();
    });

    it('should validate input params', async () => {
      const { storage } = transactionsSyncWorker;

      // Invalid chain height
      storage.getDefaultChainStore().state.chainHeight = true;
      await expect(transactionsSyncWorker.onStart())
        .to.be.rejectedWith('Chain height is not a number: "true"');

      storage.getDefaultChainStore().state.chainHeight = 0;
      await expect(transactionsSyncWorker.onStart())
        .to.be.rejectedWith('Invalid current chain height 0');
      storage.getDefaultChainStore().state.chainHeight = CHAIN_HEIGHT;

      // lastSyncedBlockHeight exceeds chain height
      storage.getDefaultChainStore().state.lastSyncedBlockHeight = CHAIN_HEIGHT * 2;
      await expect(transactionsSyncWorker.onStart())
        .to.be.rejectedWith('Start block height 2000 is greater than chain height 1000');
    });
  });
});
