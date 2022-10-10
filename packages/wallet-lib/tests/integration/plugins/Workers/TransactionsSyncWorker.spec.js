const { Transaction } = require('@dashevo/dashcore-lib');
const { expect } = require('chai');
const TransactionsSyncWorker = require('../../../../src/plugins/Workers/TransactionsSyncWorker/TransactionsSyncWorker');
const createTransportFromOptions = require('../../../../src/transport/createTransportFromOptions');
const TxStreamMock = require('../../../../src/test/mocks/TxStreamMock');
const { Wallet } = require('../../../../src');
const LocalForageAdapterMock = require('../../../../src/test/mocks/LocalForageAdapterMock');
const { waitOneTick } = require('../../../../src/test/utils');
const mockMerkleBlock = require('../../../../src/test/mocks/mockMerkleBlock');
const EVENTS = require('../../../../src/EVENTS');

describe('TransactionsSyncWorker', () => {
  const WALLET_HD_KEY = 'xprv9s21ZrQH143K4PgfRZPuYjYUWRZkGfEPuWTEUESMoEZLC274ntC4G49qxgZJEPgmujsmY52eVggtwZgJPrWTMXmbYgqDVySWg46XzbGXrSZ';
  const NETWORK = 'mainnet';
  const CHAIN_HEIGHT = 1000;

  let transactionsSyncWorker;
  let wallet;
  let historicalStream = null;
  let continuousStream = null;

  const createWorker = async (sinon, opts = {}) => {
    const defaultOptions = {
      withAdapter: false,
    };

    const options = { ...defaultOptions, ...opts };

    // Create worker
    const worker = new TransactionsSyncWorker({
      executeOnStart: false,
    });

    // Integrate worker with wallet
    wallet = new Wallet({
      offlineMode: true,
      plugins: [worker],
      allowSensitiveOperations: true,
      HDPrivateKey: WALLET_HD_KEY,
      network: NETWORK,
      adapter: options.withAdapter ? new LocalForageAdapterMock() : null,
      storage: {
        autosaveIntervalTime: 500,
      },
    });

    // Mock transport because a default one is not created in offlineMode
    wallet.transport = createTransportFromOptions({
      dapiAddresses: ['127.0.0.1'],
    });

    return worker;
  };

  const mockTxStream = (sinon) => {
    // Mock TX stream
    sinon.stub(wallet.transport.client.core, 'subscribeToTransactionsWithProofs')
      .callsFake(async (bloomFilter, rangeOpts) => {
        const { count } = rangeOpts;

        const streamMock = new TxStreamMock(sinon);

        if (count === 0) {
          continuousStream = streamMock;
        } else {
          historicalStream = streamMock;
        }
        return streamMock;
      });
  };

  const sendHistoricalTransactionsWithProofs = ({
    toAddresses,
    atHeight,
    prevMerkleBlock,
  }) => {
    // Mock Transactions
    const transactions = toAddresses
      .map((address) => new Transaction().to(address, 1000));

    const merkleBlock = mockMerkleBlock(
      transactions.map((tx) => tx.hash),
      prevMerkleBlock ? prevMerkleBlock.header : null,
    );

    const chainStore = wallet.storage.getDefaultChainStore();
    const metadata = {
      height: atHeight,
      time: merkleBlock.header.time,
    };

    chainStore.state.headersMetadata.set(merkleBlock.header.hash, metadata);

    historicalStream.sendTransactions(transactions);
    historicalStream.sendMerkleBlock(merkleBlock);

    return { transactions, merkleBlock };
  };

  const sendNewTransactions = ({ toAddresses }) => {
    // Pick addresses that will trigger gap filling
    const transactions = toAddresses
      .map((address) => new Transaction().to(address, 1000));

    continuousStream.sendTransactions(transactions);

    return { transactions };
  };

  const sendNewMerkleBlock = ({ forTransactions, atHeight, prevMerkleBlock }) => {
    // Mock Merkle Block
    const merkleBlock = mockMerkleBlock(
      forTransactions.map((tx) => tx.hash),
      prevMerkleBlock ? prevMerkleBlock.header : null,
    );

    // Prepare header metadata for merkle block
    const chainStore = wallet.storage.getDefaultChainStore();
    const metadata = {
      height: atHeight,
      time: merkleBlock.header.time,
    };
    chainStore.state.headersMetadata.set(merkleBlock.header.hash, metadata);

    continuousStream.sendMerkleBlock(merkleBlock);

    return { merkleBlock };
  };

  describe('Basic synchronization', () => {
    let account;
    let onStartPromise;

    const allTransactions = [];
    const allMerkleBlocks = [];

    before(async function before() {
      transactionsSyncWorker = await createWorker(this.sinon);
      transactionsSyncWorker.on('error', console.error);

      // Call get account in order to inject TransactionsSyncWorker plugin and its deps
      account = await wallet.getAccount();
    });

    beforeEach(async function beforeEach() {
      mockTxStream(this.sinon);
    });

    context('Historical sync', () => {
      before(async () => {
        // Set current chain height
        wallet.storage.getDefaultChainStore().updateChainHeight(CHAIN_HEIGHT);
      });

      it('should process first transactions and a merkle block', async () => {
        // Start historical sync
        onStartPromise = transactionsSyncWorker.onStart();
        await waitOneTick();

        const toAddresses = [
          account.getAddress(0).address,
          account.getAddress(1).address,
          account.getAddress(2).address,
        ];

        const { transactions, merkleBlock } = sendHistoricalTransactionsWithProofs({
          toAddresses,
          atHeight: CHAIN_HEIGHT - 3,
        });

        allTransactions.push(...transactions);
        allMerkleBlocks.push(merkleBlock);

        const accountTransactions = account.getTransactions();
        const internalAddresses = account.getAddresses('internal');
        const externalAddresses = account.getAddresses('external');

        expect(Object.keys(accountTransactions))
          .to.deep.equal(allTransactions.map((tx) => tx.hash));
        expect(Object.keys(internalAddresses).length).to.equal(20);
        expect(Object.keys(externalAddresses).length).to.equal(23);
      });

      it('should process last transactions and merkle block', async () => {
        const toAddresses = [
          account.getAddress(21).address,
          account.getAddress(22).address,
        ];

        const { transactions, merkleBlock } = sendHistoricalTransactionsWithProofs({
          toAddresses,
          atHeight: CHAIN_HEIGHT - 2,
          prevMerkleBlock: allMerkleBlocks[allMerkleBlocks.length - 1],
        });
        allTransactions.push(...transactions);
        allMerkleBlocks.push(merkleBlock);

        const accountTransactions = account.getTransactions();
        const internalAddresses = account.getAddresses('internal');
        const externalAddresses = account.getAddresses('external');

        expect(Object.keys(accountTransactions))
          .to.deep.equal(allTransactions.map((tx) => tx.hash));
        expect(Object.keys(internalAddresses).length).to.equal(20);
        expect(Object.keys(externalAddresses).length).to.equal(43);
      });

      it('should handle stream end', async () => {
        historicalStream.end();
        await onStartPromise;

        const chainStore = wallet.storage.getDefaultChainStore();
        expect(chainStore.state.lastSyncedBlockHeight).to.equal(CHAIN_HEIGHT - 1);
      });
    });

    context('Continuous sync', () => {
      context('2 TXs in the same block', () => {
        let transactions;
        it('should process unconfirmed transaction', async () => {
          await transactionsSyncWorker.execute();

          const toAddresses = [
            account.getAddress(23).address,
            account.getAddress(24).address,
          ];

          ({ transactions } = sendNewTransactions({
            toAddresses,
          }));
          allTransactions.push(...transactions);

          const accountTransactions = account.getTransactions();
          const internalAddresses = account.getAddresses('internal');
          const externalAddresses = account.getAddresses('external');

          expect(Object.keys(accountTransactions).sort())
            .to.deep.equal(allTransactions.map((tx) => tx.hash).sort());
          expect(Object.keys(internalAddresses).length).to.equal(20);
          expect(Object.keys(externalAddresses).length).to.equal(45);
        });

        it('should confirm transactions with a merkle block', async () => {
          // Mock Merkle Block
          const merkleBlockHeight = CHAIN_HEIGHT + 1;
          sendNewMerkleBlock({
            forTransactions: transactions,
            atHeight: merkleBlockHeight,
          });

          const chainStore = wallet.storage.getDefaultChainStore();
          expect(chainStore.state.lastSyncedBlockHeight)
            .to.equal(merkleBlockHeight);
        });
      });

      context('2 TXs in 2 blocks', () => {
        let transactions;

        it('should process two unconfirmed transactions', async () => {
          const toAddresses = [
            account.getAddress(43).address,
            account.getAddress(44).address,
          ];

          ({ transactions } = sendNewTransactions({
            toAddresses,
          }));
          allTransactions.push(...transactions);

          const accountTransactions = account.getTransactions();
          const internalAddresses = account.getAddresses('internal');
          const externalAddresses = account.getAddresses('external');

          expect(Object.keys(accountTransactions).sort())
            .to.deep.equal(allTransactions.map((tx) => tx.hash).sort());
          expect(Object.keys(internalAddresses).length).to.equal(20);
          expect(Object.keys(externalAddresses).length).to.equal(65);
        });

        it('should process first TX in the first merkle block', () => {
          const merkleBlockHeight = CHAIN_HEIGHT + 2;
          const { merkleBlock } = sendNewMerkleBlock({
            forTransactions: [transactions[0]],
            atHeight: merkleBlockHeight,
          });

          const chainStore = wallet.storage.getDefaultChainStore();
          const transactionsWithMetadata = chainStore.state.transactions;
          const { metadata: firstTxMetadata } = transactionsWithMetadata
            .get(transactions[0].hash);
          const { metadata: secondTxMetadata } = transactionsWithMetadata
            .get(transactions[1].hash);

          expect(firstTxMetadata).to.include({
            blockHash: merkleBlock.header.hash,
            height: merkleBlockHeight,
          });

          expect(secondTxMetadata).to.include({
            blockHash: null,
            height: null,
            time: null,
          });

          expect(chainStore.state.lastSyncedBlockHeight)
            .to.equal(merkleBlockHeight);
        });

        it('should process second TX in the second merkle block', () => {
          const merkleBlockHeight = CHAIN_HEIGHT + 3;

          const { merkleBlock } = sendNewMerkleBlock({
            forTransactions: [transactions[1]],
            atHeight: merkleBlockHeight,
          });

          // Prepare header metadata for merkle block
          const chainStore = wallet.storage.getDefaultChainStore();

          const transactionsWithMetadata = chainStore.state.transactions;
          const { metadata: secondTxMetadata } = transactionsWithMetadata
            .get(transactions[1].hash);
          expect(secondTxMetadata).to.include({
            blockHash: merkleBlock.header.hash,
            height: merkleBlockHeight,
          });
          expect(chainStore.state.lastSyncedBlockHeight)
            .to.equal(merkleBlockHeight);
        });
      });
    });
  });

  context('Synchronization with storage adapter', () => {
    let account;
    let onStartPromise;
    const allTransactions = [];
    const allMerkleBlocks = [];

    before(async function before() {
      transactionsSyncWorker = await createWorker(this.sinon, { withAdapter: true });
      transactionsSyncWorker.on('error', console.error);
    });

    beforeEach(async function beforeEach() {
      mockTxStream(this.sinon);
    });

    context('First launch', () => {
      it('should start historical sync', async () => {
        // Call get account in order to inject TransactionsSyncWorker plugin and its deps
        account = await wallet.getAccount();

        wallet.storage.getDefaultChainStore().updateChainHeight(CHAIN_HEIGHT);

        // Start historical sync
        transactionsSyncWorker.onStart().catch(console.error);
        await waitOneTick();

        const toAddresses = [
          account.getAddress(0).address,
          account.getAddress(1).address,
          account.getAddress(2).address,
        ];

        const { transactions, merkleBlock } = sendHistoricalTransactionsWithProofs({
          toAddresses,
          atHeight: Math.round(CHAIN_HEIGHT / 2),
        });
        allTransactions.push(...transactions);
        allMerkleBlocks.push(merkleBlock);

        const accountTransactions = account.getTransactions();
        const internalAddresses = account.getAddresses('internal');
        const externalAddresses = account.getAddresses('external');

        expect(Object.keys(accountTransactions))
          .to.deep.equal(allTransactions.map((tx) => tx.hash));
        expect(Object.keys(internalAddresses).length).to.equal(20);
        expect(Object.keys(externalAddresses).length).to.equal(23);
      });

      it('should stop sync', async () => {
        await new Promise((resolve) => {
          wallet.storage.on(EVENTS.SAVE_STATE_SUCCESS, resolve);
        });

        await wallet.disconnect();
        wallet.accounts = [];
        const { storage } = wallet;
        storage.reset();

        const chainStore = storage.getDefaultChainStore();
        expect(chainStore.state.transactions.size).to.equal(0);
        expect(transactionsSyncWorker.syncState)
          .to.equal(TransactionsSyncWorker.STATES.IDLE);
      });
    });

    context('Second launch', () => {
      it('should finish historical sync after restart', async () => {
        const { storage } = wallet;
        await storage.rehydrateState();
        storage.startWorker();

        account = await wallet.getAccount();
        onStartPromise = transactionsSyncWorker.onStart().catch(console.error);
        await waitOneTick();

        const toAddresses = [
          account.getAddress(21).address,
          account.getAddress(22).address,
        ];

        const { transactions, merkleBlock } = sendHistoricalTransactionsWithProofs({
          toAddresses,
          atHeight: CHAIN_HEIGHT - 1,
        });
        allTransactions.push(...transactions);
        allMerkleBlocks.push(merkleBlock);

        historicalStream.end();
        await onStartPromise;

        const accountTransactions = account.getTransactions();
        const internalAddresses = account.getAddresses('internal');
        const externalAddresses = account.getAddresses('external');

        expect(Object.keys(accountTransactions))
          .to.deep.equal(allTransactions.map((tx) => tx.hash));
        expect(Object.keys(internalAddresses).length).to.equal(20);
        expect(Object.keys(externalAddresses).length).to.equal(43);
      });

      it('should proceed with the continuous sync', async () => {
        await transactionsSyncWorker.execute();

        const toAddresses = [
          account.getAddress(23).address,
          account.getAddress(24).address,
        ];
        // Pick addresses that will trigger gap filling
        const { transactions } = sendNewTransactions({
          toAddresses,
        });
        allTransactions.push(...transactions);

        const { merkleBlock } = sendNewMerkleBlock({
          forTransactions: transactions,
          atHeight: CHAIN_HEIGHT + 1,
        });
        allMerkleBlocks.push(merkleBlock);

        const accountTransactions = account.getTransactions();
        const internalAddresses = account.getAddresses('internal');
        const externalAddresses = account.getAddresses('external');

        expect(Object.keys(accountTransactions).sort())
          .to.deep.equal(allTransactions.map((tx) => tx.hash).sort());
        expect(Object.keys(internalAddresses).length).to.equal(20);
        expect(Object.keys(externalAddresses).length).to.equal(45);
      });

      it('should stop sync', async () => {
        await new Promise((resolve) => {
          wallet.storage.on(EVENTS.SAVE_STATE_SUCCESS, resolve);
        });

        const { storage } = wallet;
        const chainStore = storage.getDefaultChainStore();

        expect(chainStore.state.lastSyncedBlockHeight)
          .to.equal(CHAIN_HEIGHT + 1);

        await wallet.disconnect();
        wallet.accounts = [];
        storage.reset();

        expect(chainStore.state.transactions.size).to.equal(0);
        expect(transactionsSyncWorker.syncState)
          .to.equal(TransactionsSyncWorker.STATES.IDLE);
      });
    });
  });
});
