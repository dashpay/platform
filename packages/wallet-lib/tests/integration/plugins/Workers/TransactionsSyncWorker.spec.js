const { Transaction } = require('@dashevo/dashcore-lib');
const { expect } = require('chai');
const TransactionsSyncWorker = require('../../../../src/plugins/Workers/TransactionsSyncWorker/TransactionsSyncWorker');
const createTransportFromOptions = require('../../../../src/transport/createTransportFromOptions');
const TxStreamMock = require('../../../../src/test/mocks/TxStreamMock');
const { Wallet } = require('../../../../src');
const LocalForageAdapterMock = require('../../../../src/test/mocks/LocalForageAdapterMock');
const { waitOneTick } = require('../../../../src/test/utils');
const mockMerkleBlock = require('../../../../src/test/mocks/mockMerkleBlock');

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
    });

    // Mock transport because a default one is not created in offlineMode
    wallet.transport = createTransportFromOptions({
      dapiAddresses: ['127.0.0.1'],
    });

    return worker;
  };

  describe('Basic synchronization', () => {
    let account;
    let onStartPromise;
    const allTransactions = [];
    const merkleBlocks = [];

    before(async function before() {
      transactionsSyncWorker = await createWorker(this.sinon);
      transactionsSyncWorker.on('error', console.error);

      // Call get account in order to inject TransactionsSyncWorker plugin and its deps
      account = await wallet.getAccount();
    });

    beforeEach(async function beforeEach() {
      // Mock TX stream
      this.sinon.stub(wallet.transport.client.core, 'subscribeToTransactionsWithProofs')
        .callsFake(async (bloomFilter, rangeOpts) => {
          const { count } = rangeOpts;

          const streamMock = new TxStreamMock(this.sinon);

          if (count === 0) {
            continuousStream = streamMock;
          } else {
            historicalStream = streamMock;
          }
          return streamMock;
        });
    });

    context('Historical sync', () => {
      before(async () => {
        // Set current chain height
        wallet.storage.getDefaultChainStore().updateChainHeight(CHAIN_HEIGHT);
      });

      it('should process first transactions and a merkle block', async () => {
        // Start historical sync
        onStartPromise = transactionsSyncWorker.onStart();// .catch(console.error);
        await waitOneTick();

        const addresses = [
          account.getAddress(0).address,
          account.getAddress(1).address,
          account.getAddress(2).address,
        ];

        // Mock Transactions
        const transactions = addresses.map((address) => new Transaction().to(address, 1000));
        allTransactions.push(...transactions);
        // Mock Merkle Block
        const merkleBlock = mockMerkleBlock(transactions.map((tx) => tx.hash));
        merkleBlocks.push(merkleBlock);
        const merkleBlockHeight = CHAIN_HEIGHT - 3;

        // Prepare header metadata for merkle block
        const chainStore = wallet.storage.getDefaultChainStore();
        const metadata = {
          height: merkleBlockHeight,
          time: merkleBlock.header.time,
        };
        chainStore.state.headersMetadata.set(merkleBlock.header.hash, metadata);

        historicalStream.sendTransactions(transactions);
        historicalStream.sendMerkleBlock(merkleBlock);

        await waitOneTick();

        const accountTransactions = account.getTransactions();
        const internalAddresses = account.getAddresses('internal');
        const externalAddresses = account.getAddresses('external');

        expect(Object.keys(accountTransactions))
          .to.deep.equal(allTransactions.map((tx) => tx.hash));
        expect(Object.keys(internalAddresses).length).to.equal(20);
        expect(Object.keys(externalAddresses).length).to.equal(23);
      });

      it('should process last transactions and merkle block', async () => {
        const addresses = [
          account.getAddress(21).address,
          account.getAddress(22).address,
        ];

        // Mock Transactions
        const transactions = addresses.map((address) => new Transaction().to(address, 1000));
        allTransactions.push(...transactions);
        // Mock Merkle Block
        const merkleBlock = mockMerkleBlock(
          transactions.map((tx) => tx.hash),
          merkleBlocks[0].header,
        );
        merkleBlocks.push(merkleBlock);
        const merkleBlockHeight = CHAIN_HEIGHT - 2;

        // Prepare header metadata for merkle block
        const chainStore = wallet.storage.getDefaultChainStore();
        const metadata = {
          height: merkleBlockHeight,
          time: merkleBlock.header.time,
        };
        chainStore.state.headersMetadata.set(merkleBlock.header.hash, metadata);

        historicalStream.sendTransactions(transactions);
        historicalStream.sendMerkleBlock(merkleBlock);

        await waitOneTick();

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

          // Pick addresses that will trigger gap filling
          transactions = [
            new Transaction()
              .to(account.getAddress(23).address, 1e8),
            new Transaction()
              .to(account.getAddress(24).address, 1e8),
          ];
          allTransactions.push(...transactions);

          continuousStream.sendTransactions(transactions);

          await waitOneTick();

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
          const merkleBlock = mockMerkleBlock(
            transactions.map((tx) => tx.hash),
            merkleBlocks[1].header,
          );
          merkleBlocks.push(merkleBlock);
          const merkleBlockHeight = CHAIN_HEIGHT + 1;

          // Prepare header metadata for merkle block
          const chainStore = wallet.storage.getDefaultChainStore();
          const metadata = {
            height: merkleBlockHeight,
            time: merkleBlock.header.time,
          };
          chainStore.state.headersMetadata.set(merkleBlock.header.hash, metadata);

          continuousStream.sendMerkleBlock(merkleBlock);

          expect(chainStore.state.lastSyncedBlockHeight)
            .to.equal(merkleBlockHeight);
        });
      });

      context('2 TXs in 2 blocks', () => {
        let transactions;

        it('should process two unconfirmed transactions', async () => {
          // Pick addresses that will trigger gap filling
          transactions = [
            new Transaction()
              .to(account.getAddress(43).address, 1e8),
            new Transaction()
              .to(account.getAddress(44).address, 1e8),
          ];
          allTransactions.push(...transactions);

          continuousStream.sendTransactions(transactions);

          await waitOneTick();

          const accountTransactions = account.getTransactions();
          const internalAddresses = account.getAddresses('internal');
          const externalAddresses = account.getAddresses('external');

          expect(Object.keys(accountTransactions).sort())
            .to.deep.equal(allTransactions.map((tx) => tx.hash).sort());
          expect(Object.keys(internalAddresses).length).to.equal(20);
          expect(Object.keys(externalAddresses).length).to.equal(65);
        });

        it('should process first TX in the first merkle block', () => {
          // Mock Merkle Block
          const merkleBlock = mockMerkleBlock(
            [transactions[0].hash],
            merkleBlocks[2].header,
          );
          merkleBlocks.push(merkleBlock);
          const merkleBlockHeight = CHAIN_HEIGHT + 2;

          // Prepare header metadata for merkle block
          const chainStore = wallet.storage.getDefaultChainStore();
          const metadata = {
            height: merkleBlockHeight,
            time: merkleBlock.header.time,
          };
          chainStore.state.headersMetadata.set(merkleBlock.header.hash, metadata);

          continuousStream.sendMerkleBlock(merkleBlock);

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
          // Mock Merkle Block
          const merkleBlock = mockMerkleBlock(
            [transactions[1].hash],
            merkleBlocks[3].header,
          );
          merkleBlocks.push(merkleBlock);
          const merkleBlockHeight = CHAIN_HEIGHT + 3;

          // Prepare header metadata for merkle block
          const chainStore = wallet.storage.getDefaultChainStore();
          const metadata = {
            height: merkleBlockHeight,
            time: merkleBlock.header.time,
          };
          chainStore.state.headersMetadata.set(merkleBlock.header.hash, metadata);

          continuousStream.sendMerkleBlock(merkleBlock);

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
    context('First launch', () => {
      it('should start historical sync and stop in the middle');
    });

    context('Second launch', () => {
      it('should finish historical sync after restart');
      it('should proceed with the continuous sync');
    });

    context('Third launch', () => {
      it('should sync up to the latest chain height after restart');
      it('should proceed with the continuous sync');
    });
  });
});
