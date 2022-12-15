const { expect } = require('chai');
const EventEmitter = require('events');
const { Transaction } = require('@dashevo/dashcore-lib');
const TransactionsSyncWorker = require('./TransactionsSyncWorker');
const TransactionsReader = require('./TransactionsReader');
const { waitOneTick } = require('../../../test/utils');
const { mockMerkleBlock } = require('../../../test/mocks/dashcore/block');
const EVENTS = require('../../../EVENTS');
const logger = require('../../../logger');

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
    worker.network = 'livenet';
    worker.logger = logger;

    worker.keyChainStore = {
      getKeyChains: () => ADDRESSES.map((addresses) => ({
        getWatchedAddresses: () => addresses,
      })),
    };

    worker.parentEvents = new EventEmitter();
    sinon.spy(worker.parentEvents, 'emit');
    sinon.spy(worker.parentEvents, 'removeListener');
    sinon.spy(worker, 'scheduleProgressUpdate');

    worker.importTransactions = sinon.stub().returns([]);

    const transactionsReader = new EventEmitter();
    transactionsReader.startHistoricalSync = sinon.spy();
    transactionsReader.startContinuousSync = sinon.spy();
    transactionsReader.stopHistoricalSync = sinon.spy();
    transactionsReader.stopContinuousSync = sinon.spy();
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
          const chainStoreState = {
            chainHeight: CHAIN_HEIGHT,
            lastSyncedBlockHeight: -1,
            headersMetadata: new Map(),
            transactions: new Map(),
          };
          this.defaultChainStore = {
            state: chainStoreState,
            pruneHeadersMetadata: sinon.spy(),
            updateLastSyncedBlockHeight: sinon.spy(),
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

    it('should return last synced block height + 1 if present', () => {
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

    context('Startup', () => {
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
      it('should kickstart reading of historical headers', async () => {
        const startPromise = transactionsSyncWorker.onStart();
        await waitOneTick();

        const { transactionsReader } = transactionsSyncWorker;

        expect(transactionsSyncWorker.syncState).to
          .equal(TransactionsSyncWorker.STATES.HISTORICAL_SYNC);
        expect(transactionsReader.on).to
          .have.been.calledWith(
            TransactionsReader.EVENTS.HISTORICAL_TRANSACTIONS,
            transactionsSyncWorker.historicalTransactionsHandler,
          );
        expect(transactionsReader.on).to
          .have.been.calledWith(
            TransactionsReader.EVENTS.MERKLE_BLOCK,
            transactionsSyncWorker.historicalMerkleBlockHandler,
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
        expect(transactionsSyncWorker.syncCheckpoint).to.equal(CHAIN_HEIGHT);
        expect(chainStore.updateLastSyncedBlockHeight).to.have.been.calledWith(CHAIN_HEIGHT);
        expect(chainStore.pruneHeadersMetadata).to.have.been.calledWith(CHAIN_HEIGHT);
        expect(storage.saveState).to.have.been.calledOnce();
        expect(transactionsReader.startHistoricalSync).to.have.not.been.called();
      });
    });

    context('Paused', () => {
      it('should handle stopped event from transactionsReader', async () => {
        const startPromise = transactionsSyncWorker.onStart();
        await waitOneTick();

        const tx = new Transaction()
          .to(ADDRESSES_KEYCHAIN_1[0], 1000);
        transactionsSyncWorker.historicalTransactionsToVerify.set(tx.hash, tx);

        const { transactionsReader } = transactionsSyncWorker;
        transactionsReader.emit(TransactionsReader.EVENTS.STOPPED);

        const {
          historicalTransactionsHandler,
          historicalMerkleBlockHandler,
          historicalDataObtainedHandler,
          transactionsReaderErrorHandler,
        } = transactionsSyncWorker;
        expect(transactionsReader.removeListener)
          .to.have.been.calledWith(
            TransactionsReader.EVENTS.HISTORICAL_TRANSACTIONS,
            historicalTransactionsHandler,
          );
        expect(transactionsReader.removeListener)
          .to.have.been.calledWith(
            TransactionsReader.EVENTS.MERKLE_BLOCK,
            historicalMerkleBlockHandler,
          );
        expect(transactionsReader.removeListener)
          .to.have.been.calledWith(
            TransactionsReader.EVENTS.HISTORICAL_DATA_OBTAINED,
            historicalDataObtainedHandler,
          );
        expect(transactionsReader.removeListener)
          .to.have.been.calledWith(
            TransactionsReader.EVENTS.ERROR,
            transactionsReaderErrorHandler,
          );
        expect(transactionsSyncWorker.historicalTransactionsToVerify.size).to.equal(0);

        await startPromise;
      });

      it('should start over from the sync checkpoint if historical sync is interrupted', async () => {
        const { transactionsReader } = transactionsSyncWorker;

        // Start historical sync
        let startPromise = transactionsSyncWorker.onStart();
        await waitOneTick();

        // Put on a pause
        const syncCheckpoint = CHAIN_HEIGHT - 500;
        transactionsSyncWorker.syncCheckpoint = syncCheckpoint;
        transactionsReader.emit(TransactionsReader.EVENTS.STOPPED);
        await startPromise;

        // Continue historical sync
        startPromise = transactionsSyncWorker.onStart();
        await waitOneTick();
        transactionsReader.emit(TransactionsReader.EVENTS.STOPPED);
        await startPromise;

        const { secondCall } = transactionsReader.startHistoricalSync;

        expect(secondCall.args).to.deep.equal([
          syncCheckpoint,
          CHAIN_HEIGHT,
          ADDRESSES.reduce((acc, addresses) => acc.concat(addresses), []),
        ]);
      });
    });

    context('Finished', () => {
      it('should prepare for continuous sync after historical data is obtained', async function test() {
        const { transactionsReader, storage } = transactionsSyncWorker;
        const chainStore = storage.getDefaultChainStore();

        transactionsSyncWorker.updateProgress = this.sinon.spy();

        const startPromise = transactionsSyncWorker.onStart();
        await waitOneTick();
        transactionsReader.emit(TransactionsReader.EVENTS.HISTORICAL_DATA_OBTAINED);
        await startPromise;

        expect(transactionsSyncWorker.syncState).to
          .equal(TransactionsSyncWorker.STATES.IDLE);
        expect(transactionsReader.removeListener)
          .to.have.been.calledWith(
            TransactionsReader.EVENTS.HISTORICAL_TRANSACTIONS,
            transactionsSyncWorker.historicalTransactionsHandler,
          );
        expect(transactionsReader.removeListener)
          .to.have.been.calledWith(
            TransactionsReader.EVENTS.MERKLE_BLOCK,
            transactionsSyncWorker.historicalMerkleBlockHandler,
          );
        expect(transactionsReader.removeListener)
          .to.have.been.calledWith(
            TransactionsReader.EVENTS.ERROR,
            transactionsSyncWorker.transactionsReaderErrorHandler,
          );
        expect(transactionsSyncWorker.syncCheckpoint).to.equal(CHAIN_HEIGHT);
        expect(chainStore.updateLastSyncedBlockHeight).to.have.been.calledWith(CHAIN_HEIGHT);
        expect(transactionsSyncWorker.updateProgress).to.have.been.calledOnce();
        expect(transactionsSyncWorker.storage.saveState).to.have.been.calledOnce();

        expect(chainStore.pruneHeadersMetadata).to.have.been.calledOnceWith(CHAIN_HEIGHT);
      });
      it('should throw an error in case there are transactions to verify left', async () => {
        const { transactionsReader } = transactionsSyncWorker;

        const tx = new Transaction()
          .to(ADDRESSES_KEYCHAIN_1[0], 1000);
        transactionsSyncWorker.historicalTransactionsToVerify.set(tx.hash, tx);

        const startPromise = transactionsSyncWorker.onStart();
        await waitOneTick();
        transactionsReader.emit(TransactionsReader.EVENTS.HISTORICAL_DATA_OBTAINED);

        await expect(startPromise)
          .to.be.rejectedWith('Historical data obtained but there are still transactions to verify');
      });
    });

    context('Error', () => {
      it('should handle error event from transactionsReader', async () => {
        const { transactionsReader } = transactionsSyncWorker;

        const startPromise = transactionsSyncWorker.onStart();
        await waitOneTick();
        const tx = new Transaction()
          .to(ADDRESSES_KEYCHAIN_1[0], 1000);
        transactionsSyncWorker.historicalTransactionsToVerify.set(tx.hash, tx);

        // Throw an error and interrupt historical sync
        const syncError = new Error('Error syncing historical data');
        transactionsReader.emit(TransactionsReader.EVENTS.ERROR, syncError);

        await expect(startPromise)
          .to.be.rejectedWith(syncError);

        expect(transactionsSyncWorker.historicalTransactionsToVerify.size)
          .to.equal(0);
      });
    });
  });

  describe('#execute', () => {
    beforeEach(function beforeEach() {
      transactionsSyncWorker = createTransactionsSyncWorker(this.sinon);
    });

    it('should kickstart continuous sync', async () => {
      const { transactionsReader } = transactionsSyncWorker;

      transactionsSyncWorker.syncCheckpoint = 1200;
      await transactionsSyncWorker.execute();

      expect(transactionsSyncWorker.syncState).to
        .equal(TransactionsSyncWorker.STATES.CONTINUOUS_SYNC);
      expect(transactionsReader.on).to
        .have.been.calledWith(
          TransactionsReader.EVENTS.NEW_TRANSACTIONS,
          transactionsSyncWorker.newTransactionsHandler,
        );
      expect(transactionsReader.on).to
        .have.been.calledWith(
          TransactionsReader.EVENTS.MERKLE_BLOCK,
          transactionsSyncWorker.newMerkleBlockHandler,
        );
      expect(transactionsReader.on).to
        .have.been.calledWith(
          TransactionsReader.EVENTS.ERROR,
          transactionsSyncWorker.transactionsReaderErrorHandler,
        );

      expect(transactionsReader.once).to
        .have.been.calledWith(
          TransactionsReader.EVENTS.STOPPED,
          transactionsSyncWorker.transactionsReaderStoppedHandler,
        );
      expect(transactionsReader.startContinuousSync).to
        .have.been.calledWith(
          1200,
          ADDRESSES.reduce((acc, addresses) => acc.concat(addresses), []),
        );
      expect(transactionsSyncWorker.syncState)
        .to.equal(TransactionsSyncWorker.STATES.CONTINUOUS_SYNC);
    });

    it('should forward an error from blockHeadersProvider', async function test() {
      transactionsSyncWorker.syncCheckpoint = 1200;
      await transactionsSyncWorker.execute();

      const errorCallback = this.sinon.spy();
      transactionsSyncWorker.parentEvents.on('error', errorCallback);
      const { transactionsReader } = transactionsSyncWorker;
      const error = new Error('Test error');
      transactionsReader.emit('error', error);

      expect(errorCallback).to.have.been.calledOnceWith(error);
    });

    it('should not allow multiple executions', async () => {
      await transactionsSyncWorker.execute();
      await expect(transactionsSyncWorker.execute()).to.be.rejected();
    });

    it('should handle stopped event from transactionsReader', async () => {
      await transactionsSyncWorker.execute();

      const { transactionsReader } = transactionsSyncWorker;
      transactionsReader.emit(TransactionsReader.EVENTS.STOPPED);

      const {
        newTransactionsHandler,
        newMerkleBlockHandler,
        transactionsReaderErrorHandler,
      } = transactionsSyncWorker;
      expect(transactionsReader.removeListener)
        .to.have.been.calledWith(
          TransactionsReader.EVENTS.NEW_TRANSACTIONS,
          newTransactionsHandler,
        );
      expect(transactionsReader.removeListener)
        .to.have.been.calledWith(
          TransactionsReader.EVENTS.MERKLE_BLOCK,
          newMerkleBlockHandler,
        );
      expect(transactionsReader.removeListener)
        .to.have.been.calledWith(
          TransactionsReader.EVENTS.ERROR,
          transactionsReaderErrorHandler,
        );
    });

    it('should start over from the sync checkpoint if continuous sync is interrupted', async () => {
      const { transactionsReader } = transactionsSyncWorker;

      // Start historical sync
      await transactionsSyncWorker.execute();

      // Put on a pause
      const syncCheckpoint = CHAIN_HEIGHT + 500;
      transactionsSyncWorker.syncCheckpoint = syncCheckpoint;
      transactionsReader.emit(TransactionsReader.EVENTS.STOPPED);

      // Continue historical sync
      await transactionsSyncWorker.execute();
      transactionsReader.emit(TransactionsReader.EVENTS.STOPPED);

      const { secondCall } = transactionsReader.startContinuousSync;

      expect(secondCall.args).to.deep.equal([
        syncCheckpoint,
        ADDRESSES.reduce((acc, addresses) => acc.concat(addresses), []),
      ]);
    });

    it('should handle error event from transactionsReader', async () => {
      const { transactionsReader } = transactionsSyncWorker;

      let emittedError = null;
      transactionsSyncWorker.parentEvents.on('error', (e) => {
        emittedError = e;
      });

      // Start continuous sync
      await transactionsSyncWorker.execute();

      // Throw an error from reader
      const syncError = new Error('Error syncing historical data');
      transactionsReader.emit(TransactionsReader.EVENTS.ERROR, syncError);

      expect(emittedError).to.equal(syncError);
    });
  });

  describe('#onStop', () => {
    beforeEach(function beforeEach() {
      transactionsSyncWorker = createTransactionsSyncWorker(this.sinon);
    });

    it('should stop historical sync', async () => {
      transactionsSyncWorker.syncState = TransactionsSyncWorker.STATES.HISTORICAL_SYNC;
      const { transactionsReader } = transactionsSyncWorker;
      await transactionsSyncWorker.onStop();
      expect(transactionsReader.stopHistoricalSync).to.have.been.calledOnce();
      expect(transactionsReader.stopContinuousSync).to.have.not.been.called();
    });

    it('should stop continuous sync', async () => {
      transactionsSyncWorker.syncState = TransactionsSyncWorker.STATES.CONTINUOUS_SYNC;
      const { transactionsReader } = transactionsSyncWorker;
      await transactionsSyncWorker.onStop();
      expect(transactionsReader.stopContinuousSync).to.have.been.calledOnce();
      expect(transactionsReader.stopHistoricalSync).to.have.not.been.called();
    });

    it('should unsubscribe from blockHeightChanged handler', async function test() {
      const handler = this.sinon.spy();
      transactionsSyncWorker.blockHeightChangedHandler = handler;
      transactionsSyncWorker.parentEvents.on(
        EVENTS.BLOCKHEIGHT_CHANGED,
        transactionsSyncWorker.blockHeightChangedHandler,
      );

      await transactionsSyncWorker.onStop();
      expect(transactionsSyncWorker.parentEvents.removeListener)
        .to.have.been.calledWith(EVENTS.BLOCKHEIGHT_CHANGED, handler);
      expect(transactionsSyncWorker.blockHeightChangedHandler).to.equal(null);
    });
  });

  describe('#historicalTransactionsHandler', () => {
    beforeEach(function beforeEach() {
      transactionsSyncWorker = createTransactionsSyncWorker(this.sinon);
    });

    it('should add transactions to the verification pool', () => {
      const transactions = ADDRESSES_KEYCHAIN_1
        .map((address) => new Transaction().to(address, 1000));

      transactionsSyncWorker.historicalTransactionsHandler(transactions);

      const expectedResult = transactions.reduce((acc, transaction) => {
        acc.set(transaction.hash, transaction);
        return acc;
      }, new Map());

      expect(transactionsSyncWorker.historicalTransactionsToVerify)
        .to.deep.equal(expectedResult);
    });

    it('should validate arguments', () => {
      expect(() => transactionsSyncWorker.historicalTransactionsHandler([]))
        .to.throw('No transactions to process');
    });
  });

  describe('#historicalMerkleBlockHandler', () => {
    let storage;
    let chainStore;
    beforeEach(function beforeEach() {
      transactionsSyncWorker = createTransactionsSyncWorker(this.sinon);
      ({ storage } = transactionsSyncWorker);
      chainStore = storage.getDefaultChainStore();
    });

    context('Accept merkle block', () => {
      it('should verify transactions in the pool and accept merkle block', function test() {
        // Create transactions
        const transactions = [
          new Transaction().to(ADDRESSES_KEYCHAIN_1[0], 1000),
          new Transaction().to(ADDRESSES_KEYCHAIN_1[1], 2000),
        ];

        // Add transactions to the verification pool
        transactions.forEach((tx) => {
          transactionsSyncWorker.historicalTransactionsToVerify.set(tx.hash, tx);
        });

        // Create merkle block
        const merkleBlock = mockMerkleBlock(transactions.map((tx) => tx.hash));
        const merkleBlockHeight = 500;

        const metadata = {
          height: merkleBlockHeight,
          time: merkleBlock.header.time,
        };
        // Update chain store
        chainStore.state.headersMetadata.set(merkleBlock.header.hash, metadata);

        // Simulate addresses gap fill
        transactionsSyncWorker.importTransactions.returns({
          addressesGenerated: [ADDRESSES_KEYCHAIN_1[2]],
        });

        // Prepare event handler payload
        const dataEventPayload = {
          merkleBlock,
          acceptMerkleBlock: this.sinon.spy(),
          rejectMerkleBlock: this.sinon.spy(),
        };

        transactionsSyncWorker.historicalMerkleBlockHandler(dataEventPayload);

        const expectedMetadata = {
          blockHash: merkleBlock.header.hash,
          height: metadata.height,
          time: new Date(metadata.time * 1000),
          instantLocked: false,
          chainLocked: false,
        };

        expect(transactionsSyncWorker.importTransactions)
          .to.have.been.calledWith(transactions.map((tx) => [tx, expectedMetadata]));

        expect(dataEventPayload.rejectMerkleBlock).to.have.not.been.called();
        expect(dataEventPayload.acceptMerkleBlock)
          .to.have.been.calledWith(merkleBlockHeight, [ADDRESSES_KEYCHAIN_1[2]]);
        expect(transactionsSyncWorker.syncCheckpoint).to.equal(merkleBlockHeight);
        expect(chainStore.pruneHeadersMetadata).to.have.been.calledWith(merkleBlockHeight);
        expect(chainStore.updateLastSyncedBlockHeight).to.have.been.calledWith(merkleBlockHeight);
        expect(storage.scheduleStateSave).to.have.been.called();
        expect(transactionsSyncWorker.scheduleProgressUpdate).to.have.been.called();
      });
    });

    context('Reject merkle block', () => {
      it('should reject in case header metadata is missing', function test() {
        // Create transactions
        const transactions = [
          new Transaction().to(ADDRESSES_KEYCHAIN_1[0], 1000),
          new Transaction().to(ADDRESSES_KEYCHAIN_1[1], 2000),
        ];

        // Add transactions to the verification pool
        transactions.forEach((tx) => {
          transactionsSyncWorker.historicalTransactionsToVerify.set(tx.hash, tx);
        });

        const merkleBlock = mockMerkleBlock(transactions.map((tx) => tx.hash));

        // Prepare event handler payload
        const dataEventPayload = {
          merkleBlock,
          acceptMerkleBlock: this.sinon.spy(),
          rejectMerkleBlock: this.sinon.spy(),
        };

        transactionsSyncWorker.historicalMerkleBlockHandler(dataEventPayload);

        expect(dataEventPayload.acceptMerkleBlock).to.have.not.been.called();
        const { args } = dataEventPayload.rejectMerkleBlock.getCall(0);
        expect(args[0].message)
          .to.equal('Header metadata was not found during the merkle block processing');

        expect(chainStore.updateLastSyncedBlockHeight).to.have.not.been.called();
        expect(chainStore.pruneHeadersMetadata).to.have.not.been.called();
        expect(storage.scheduleStateSave).to.have.not.been.called();
        expect(transactionsSyncWorker.scheduleProgressUpdate).to.have.not.been.called();
      });

      it('should reject in case of invalid header time', function test() {
        // Create transactions
        const transactions = [
          new Transaction().to(ADDRESSES_KEYCHAIN_1[0], 1000),
        ];

        // Add transactions to the verification pool
        transactions.forEach((tx) => {
          transactionsSyncWorker.historicalTransactionsToVerify.set(tx.hash, tx);
        });

        const merkleBlock = mockMerkleBlock(transactions.map((tx) => tx.hash));
        const merkleBlockHeight = 500;

        // Update chain store
        chainStore.state.headersMetadata.set(merkleBlock.header.hash, {
          height: merkleBlockHeight,
          time: 0,
        });

        // Prepare event handler payload
        const dataEventPayload = {
          merkleBlock,
          acceptMerkleBlock: this.sinon.spy(),
          rejectMerkleBlock: this.sinon.spy(),
        };

        transactionsSyncWorker.historicalMerkleBlockHandler(dataEventPayload);

        expect(dataEventPayload.acceptMerkleBlock).to.have.not.been.called();
        const { args } = dataEventPayload.rejectMerkleBlock.getCall(0);
        expect(args[0].message)
          .to.equal('Invalid header time: 0');
        expect(chainStore.updateLastSyncedBlockHeight).to.have.not.been.called();
        expect(chainStore.pruneHeadersMetadata).to.have.not.been.called();
        expect(storage.scheduleStateSave).to.have.not.been.called();
        expect(transactionsSyncWorker.scheduleProgressUpdate).to.have.not.been.called();
      });

      it('should reject in case of invalid header height', function test() {
        // Create transactions
        const transactions = [
          new Transaction().to(ADDRESSES_KEYCHAIN_1[0], 1000),
        ];

        // Add transactions to the verification pool
        transactions.forEach((tx) => {
          transactionsSyncWorker.historicalTransactionsToVerify.set(tx.hash, tx);
        });

        const merkleBlock = mockMerkleBlock(transactions.map((tx) => tx.hash));

        // Update chain store
        chainStore.state.headersMetadata.set(merkleBlock.header.hash, {
          height: -1,
          time: merkleBlock.header.time,
        });

        // Prepare event handler payload
        const dataEventPayload = {
          merkleBlock,
          acceptMerkleBlock: this.sinon.spy(),
          rejectMerkleBlock: this.sinon.spy(),
        };

        transactionsSyncWorker.historicalMerkleBlockHandler(dataEventPayload);

        expect(dataEventPayload.acceptMerkleBlock).to.have.not.been.called();
        const { args } = dataEventPayload.rejectMerkleBlock.getCall(0);
        expect(args[0].message)
          .to.equal('Invalid header height: -1');
        expect(chainStore.updateLastSyncedBlockHeight).to.have.not.been.called();
        expect(chainStore.pruneHeadersMetadata).to.have.not.been.called();
        expect(storage.scheduleStateSave).to.have.not.been.called();
        expect(transactionsSyncWorker.scheduleProgressUpdate).to.have.not.been.called();
      });

      it('should reject if tx hash from verification pool not present in the merkle block', function test() {
        // Create transactions
        const transactions = [
          new Transaction().to(ADDRESSES_KEYCHAIN_1[0], 1000),
          new Transaction().to(ADDRESSES_KEYCHAIN_1[1], 2000),
          new Transaction().to(ADDRESSES_KEYCHAIN_1[2], 3000),
        ];

        // Add transactions to the verification pool
        transactions.forEach((tx) => {
          transactionsSyncWorker.historicalTransactionsToVerify.set(tx.hash, tx);
        });

        // Create merkle block
        const merkleBlock = mockMerkleBlock(transactions.slice(1).map((tx) => tx.hash));
        const merkleBlockHeight = 500;

        // Update chain store
        chainStore.state.headersMetadata.set(merkleBlock.header.hash, {
          height: merkleBlockHeight,
          time: merkleBlock.header.time,
        });

        // Prepare event handler payload
        const dataEventPayload = {
          merkleBlock,
          acceptMerkleBlock: this.sinon.spy(),
          rejectMerkleBlock: this.sinon.spy(),
        };

        transactionsSyncWorker.historicalMerkleBlockHandler(dataEventPayload);

        expect(dataEventPayload.acceptMerkleBlock).to.have.not.been.called();
        const { args } = dataEventPayload.rejectMerkleBlock.getCall(0);
        expect(args[0].message)
          .to.equal(`Transaction ${transactions[0].hash} was not found in merkle block ${merkleBlock.header.hash}`);
        expect(chainStore.updateLastSyncedBlockHeight).to.have.not.been.called();
        expect(chainStore.pruneHeadersMetadata).to.have.not.been.called();
        expect(storage.scheduleStateSave).to.have.not.been.called();
        expect(transactionsSyncWorker.scheduleProgressUpdate).to.have.not.been.called();
      });
    });
  });

  describe('#newTransactionHandler', () => {
    beforeEach(function beforeEach() {
      transactionsSyncWorker = createTransactionsSyncWorker(this.sinon);
    });

    it('should handle new transactions', function test() {
      const transactions = ADDRESSES_KEYCHAIN_1.slice(0, 2)
        .map((address) => new Transaction().to(address, 1000));

      // Simulate addresses gap fill
      const addressesGenerated = ADDRESSES_KEYCHAIN_1.slice(2);
      transactionsSyncWorker.importTransactions.returns({
        addressesGenerated,
      });

      // Simulate data event
      const dataEventPayload = {
        transactions,
        handleNewAddresses: this.sinon.spy(),
      };

      transactionsSyncWorker
        .newTransactionsHandler(dataEventPayload);

      const expectedResult = transactions.reduce((acc, transaction) => {
        acc.set(transaction.hash, transaction);
        return acc;
      }, new Map());

      expect(transactionsSyncWorker.importTransactions)
        .to.have.been.calledWith(transactions.map((tx) => [tx]));

      expect(transactionsSyncWorker.historicalTransactionsToVerify)
        .to.deep.equal(expectedResult);

      expect(dataEventPayload.handleNewAddresses)
        .to.have.been.calledWith(addressesGenerated);
    });

    it('should validate arguments', () => {
      expect(() => transactionsSyncWorker.newTransactionsHandler({ transactions: [] }))
        .to.throw('No new transactions to process');
    });

    it('should not handle same transactions twice', function test() {
      // Make importTransactions memorize transactions
      const chainStore = transactionsSyncWorker.storage.getDefaultChainStore();
      transactionsSyncWorker.importTransactions.callsFake((txs) => {
        txs.forEach(([tx]) => {
          chainStore.state.transactions.set(tx.hash, tx);
        });
        return [];
      });

      // Generate and handle first set of transactions
      const transactions = ADDRESSES_KEYCHAIN_1.slice(0, 2)
        .map((address) => new Transaction().to(address, 1000));

      transactionsSyncWorker.newTransactionsHandler({
        transactions,
        handleNewAddresses: this.sinon.spy(),
      });

      // Generate and handle second set of transactions with duplicated ones
      const transactionsWithDuplicates = ADDRESSES_KEYCHAIN_1
        .map((address) => new Transaction().to(address, 1000));

      transactionsSyncWorker.newTransactionsHandler({
        transactions: transactionsWithDuplicates,
        handleNewAddresses: this.sinon.spy(),
      });

      const { firstCall } = transactionsSyncWorker.importTransactions;
      const { secondCall } = transactionsSyncWorker.importTransactions;

      expect(firstCall.args)
        .to.deep.equal([transactions.map((tx) => [tx])]);

      expect(secondCall.args[0].length).to.be.greaterThan(0);
      const addedTxs = transactionsWithDuplicates.slice(transactions.length);
      expect(secondCall.args)
        .to.deep.equal([addedTxs.map((tx) => [tx])]);
    });
  });

  describe('#newMerkleBlockHandler', () => {
    let storage;
    let chainStore;
    beforeEach(function beforeEach() {
      transactionsSyncWorker = createTransactionsSyncWorker(this.sinon);
      ({ storage } = transactionsSyncWorker);
      chainStore = storage.getDefaultChainStore();
    });

    it('should not process same merkle block two times', function () {
      const merkleBlock = mockMerkleBlock([]);
      const merkleBlockHeight = 500;

      // Update chain store
      const metadata = {
        height: merkleBlockHeight,
        time: merkleBlock.header.time,
      };
      chainStore.state.headersMetadata.set(merkleBlock.header.hash, metadata);

      // Prepare event handler payload
      const dataEventPayload = {
        merkleBlock,
        acceptMerkleBlock: this.sinon.spy(),
        rejectMerkleBlock: this.sinon.spy(),
      };

      transactionsSyncWorker.newMerkleBlockHandler(dataEventPayload);
      chainStore.state.lastSyncedBlockHeight = merkleBlockHeight;
      transactionsSyncWorker.newMerkleBlockHandler(dataEventPayload);

      expect(dataEventPayload.acceptMerkleBlock).to.have.been.calledOnce();
    });

    context('Accept merkle block', () => {
      it('should verify transactions in the pool and accept merkle block', function test() {
        // Create transactions
        const transactions = [
          new Transaction().to(ADDRESSES_KEYCHAIN_1[0], 1000),
          new Transaction().to(ADDRESSES_KEYCHAIN_1[1], 2000),
        ];

        // Add transactions to the verification pool
        transactions.forEach((tx) => {
          transactionsSyncWorker.historicalTransactionsToVerify.set(tx.hash, tx);
        });

        // Create merkle block
        const merkleBlock = mockMerkleBlock(transactions.map((tx) => tx.hash));
        const merkleBlockHeight = 500;

        // Update chain store
        const metadata = {
          height: merkleBlockHeight,
          time: merkleBlock.header.time,
        };
        chainStore.state.headersMetadata.set(merkleBlock.header.hash, metadata);

        // Prepare event handler payload
        const dataEventPayload = {
          merkleBlock,
          acceptMerkleBlock: this.sinon.spy(),
          rejectMerkleBlock: this.sinon.spy(),
        };

        const expectedMetadata = {
          blockHash: merkleBlock.header.hash,
          height: metadata.height,
          time: new Date(metadata.time * 1000),
          instantLocked: false,
          chainLocked: false,
        };

        transactionsSyncWorker.newMerkleBlockHandler(dataEventPayload);

        expect(transactionsSyncWorker.importTransactions)
          .to.have.been.calledWith(transactions.map((tx) => [tx, expectedMetadata]));
        expect(dataEventPayload.rejectMerkleBlock).to.have.not.been.called();
        expect(dataEventPayload.acceptMerkleBlock)
          .to.have.been.calledWith(merkleBlockHeight);
        expect(transactionsSyncWorker.syncCheckpoint).to.equal(merkleBlockHeight);
        expect(chainStore.updateLastSyncedBlockHeight).to.have.been.calledWith(merkleBlockHeight);
        expect(chainStore.pruneHeadersMetadata).to.have.been.calledWith(merkleBlockHeight);
        expect(storage.scheduleStateSave).to.have.been.called();
        expect(transactionsSyncWorker.parentEvents.emit)
          .to.have.been.calledTwice();
        const { firstCall, secondCall } = transactionsSyncWorker.parentEvents.emit;
        expect(firstCall.args)
          .to.deep.equal([EVENTS.CONFIRMED_TRANSACTION, transactions[0]]);
        expect(secondCall.args)
          .to.deep.equal([EVENTS.CONFIRMED_TRANSACTION, transactions[1]]);
      });

      it('should verify merkle block if no relevant transactions found', function test() {
        // Create merkle block
        const merkleBlock = mockMerkleBlock([]);
        const merkleBlockHeight = 500;

        // Update chain store
        chainStore.state.headersMetadata.set(merkleBlock.header.hash, {
          height: merkleBlockHeight,
          time: merkleBlock.header.time,
        });

        // Prepare event handler payload
        const dataEventPayload = {
          merkleBlock,
          acceptMerkleBlock: this.sinon.spy(),
          rejectMerkleBlock: this.sinon.spy(),
        };

        transactionsSyncWorker.newMerkleBlockHandler(dataEventPayload);

        expect(dataEventPayload.rejectMerkleBlock).to.have.not.been.called();
        expect(dataEventPayload.acceptMerkleBlock)
          .to.have.been.calledWith(merkleBlockHeight);
        expect(transactionsSyncWorker.importTransactions)
          .to.have.not.been.called();
        expect(transactionsSyncWorker.syncCheckpoint).to.equal(merkleBlockHeight);
        expect(chainStore.updateLastSyncedBlockHeight).to.have.been.calledWith(merkleBlockHeight);
        expect(chainStore.pruneHeadersMetadata).to.have.been.calledWith(merkleBlockHeight);
        expect(storage.scheduleStateSave).to.have.been.called();
      });

      it('should retry after BLOCKHEIGHT_CHANGED event in case metadata was not found', function test() {
        // Create merkle block
        const merkleBlock = mockMerkleBlock([]);
        const merkleBlockHeight = 500;

        // Prepare event handler payload
        const dataEventPayload = {
          merkleBlock,
          acceptMerkleBlock: this.sinon.spy(),
          rejectMerkleBlock: this.sinon.spy(),
        };

        // Emit merkle block data event
        transactionsSyncWorker.newMerkleBlockHandler(dataEventPayload);

        // Update chain store and emit BLOCKHEIGHT_CHANGED event
        chainStore.state.headersMetadata.set(merkleBlock.header.hash, {
          height: merkleBlockHeight,
          time: merkleBlock.header.time,
        });

        transactionsSyncWorker.parentEvents
          .emit(EVENTS.BLOCKHEIGHT_CHANGED, merkleBlockHeight);

        expect(dataEventPayload.rejectMerkleBlock).to.have.not.been.called();
        expect(dataEventPayload.acceptMerkleBlock)
          .to.have.been.calledOnceWith(merkleBlockHeight);
        expect(chainStore.updateLastSyncedBlockHeight)
          .to.have.been.calledOnceWith(merkleBlockHeight);
        expect(chainStore.pruneHeadersMetadata).to.have.been.calledWith(merkleBlockHeight);
        expect(storage.scheduleStateSave).to.have.been.calledOnce();
        expect(transactionsSyncWorker.syncCheckpoint).to.equal(merkleBlockHeight);
      });
    });

    context('Reject merkle block', () => {
      it('should reject in case of invalid header metadata', function test() {
        const merkleBlock = mockMerkleBlock([]);
        const merkleBlockHeight = 500;

        // Update chain store
        chainStore.state.headersMetadata.set(merkleBlock.header.hash, {
          height: merkleBlockHeight,
          time: 0,
        });

        // Prepare event handler payload
        const dataEventPayload = {
          merkleBlock,
          acceptMerkleBlock: this.sinon.spy(),
          rejectMerkleBlock: this.sinon.spy(),
        };

        transactionsSyncWorker.newMerkleBlockHandler(dataEventPayload);

        expect(dataEventPayload.acceptMerkleBlock).to.have.not.been.called();
        const { args } = dataEventPayload.rejectMerkleBlock.getCall(0);
        expect(args[0].message)
          .to.equal('Invalid header time: 0');
        expect(chainStore.updateLastSyncedBlockHeight).to.have.not.been.called();
        expect(chainStore.pruneHeadersMetadata).to.have.not.been.called();
        expect(storage.scheduleStateSave).to.have.not.been.called();
      });

      it('should reject in case of invalid header height', function test() {
        const merkleBlock = mockMerkleBlock([]);

        // Update chain store
        chainStore.state.headersMetadata.set(merkleBlock.header.hash, {
          height: -1,
          time: merkleBlock.header.time,
        });

        // Prepare event handler payload
        const dataEventPayload = {
          merkleBlock,
          acceptMerkleBlock: this.sinon.spy(),
          rejectMerkleBlock: this.sinon.spy(),
        };

        transactionsSyncWorker.newMerkleBlockHandler(dataEventPayload);

        expect(dataEventPayload.acceptMerkleBlock).to.have.not.been.called();
        const { args } = dataEventPayload.rejectMerkleBlock.getCall(0);
        expect(args[0].message)
          .to.equal('Invalid header height: -1');
        expect(chainStore.updateLastSyncedBlockHeight).to.have.not.been.called();
        expect(chainStore.pruneHeadersMetadata).to.have.not.been.called();
        expect(storage.scheduleStateSave).to.have.not.been.called();
      });

      // TODO: should reject in case merkle block verification failed
    });
  });
});
