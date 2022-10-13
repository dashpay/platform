const {
  HDPrivateKey,
  Transaction,
  PrivateKey
} = require('@dashevo/dashcore-lib');

const { expect } = require('chai');

const { Wallet } = require("../../../src");
const TransactionsSyncWorker = require("../../../src/plugins/Workers/TransactionsSyncWorker/TransactionsSyncWorker");
const ChainPlugin = require("../../../src/plugins/Plugins/ChainPlugin");
const LocalForageAdapterMock = require("../../../src/test/mocks/LocalForageAdapterMock");
const {waitOneTick} = require("../../../src/test/utils");
const mockMerkleBlock = require("../../../src/test/mocks/mockMerkleBlock");
const createTransportFromOptions = require("../../../src/transport/createTransportFromOptions");
const TxStreamMock = require("../../../src/test/mocks/TxStreamMock");

describe('Wallet', () => {
  // TODO: write test that ensures that storage getting wiped after removing skipSynchronizationBeofreHeight flag
  describe('Storage', () => {
    let wallet;
    let txSyncWorker;
    let chainPlugin;
    let bestBlockHeight = 42;
    let storageAdapterMock = new LocalForageAdapterMock();
    let continuousStream;
    let historicalStream;
    const allMerkleBlocks = [];

    beforeEach(async function() {
      const testHDKey = "xprv9s21ZrQH143K4PgfRZPuYjYUWRZkGfEPuWTEUESMoEZLC274ntC4G49qxgZJEPgmujsmY52eVggtwZgJPrWTMXmbYgqDVySWg46XzbGXrSZ";
      txSyncWorker = new TransactionsSyncWorker({ executeOnStart: false });
      chainPlugin = new ChainPlugin({ executeOnStart: false });

      wallet = new Wallet({
        offlineMode: true,
        plugins: [chainPlugin, txSyncWorker],
        allowSensitiveOperations: true,
        HDPrivateKey: new HDPrivateKey(testHDKey),
        adapter: storageAdapterMock,
        network: 'livenet'
      });

      // Mock transport because a default one is not created in offlineMode
      wallet.transport = createTransportFromOptions({
        dapiAddresses: [],
      });

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

      this.sinon.stub(wallet.transport.client.core, 'getStatus')
        .resolves({
          chain: { blocksCount: bestBlockHeight },
          network: { fee: 237 }
        })

      this.sinon.stub(wallet.transport.client.core, 'broadcastTransaction')
        .callsFake(async (rawTx) => new Transaction(rawTx).hash);

      await wallet.getAccount().catch(console.error);
      await chainPlugin.onStart().catch(console.error);
    })

    /**
     * In this scenario we have a fresh wallet that receives a funding transaction
     * and sends a transaction on his own.
     * Points to check:
     * - subscr
     */
    it('should fill the storage for a fresh wallet', async function() {
      const account = await wallet.getAccount();
      const { address: addressToFund } = account.getUnusedAddress();

      /** Define a scenario */
      const scenario = {
        transactions: {
          fundingTx: new Transaction().to(addressToFund, 10000),
        },
      }

      /** Start transactions sync plugin */
      txSyncWorker.onStart().catch(console.error);
      await waitOneTick();

      /** Ensure proper transport arguments */
      // expect(transportMock.subscribeToTransactionsWithProofs.firstCall.args[1])
      //   .to.deep.equal({ fromBlockHeight: 1, count: 41 });

      /** Send first funding transaction to the wallet */
      const { fundingTx } = scenario.transactions;
      historicalStream.sendTransactions([fundingTx]);

      await waitOneTick();

      await wallet.storage.saveState();

      /** Ensure that storage has no items for transactions without the metadata */
      let storage = storageAdapterMock.getItem(`wallet_${wallet.walletId}`)
      let chainStoreState = storage.chains[wallet.network];
      expect(chainStoreState.transactions).to.be.empty;
      expect(chainStoreState.txMetadata).to.be.empty;
      // -6 to ensure reorg safe saving procedure
      expect(chainStoreState.lastSyncedBlockHeight).to.equal(-1)

      const merkleBlockFirst = mockMerkleBlock([fundingTx.hash]);
      allMerkleBlocks.push(merkleBlockFirst);
      const merkleBlockFirstHeight = 10;
      wallet.storage.getDefaultChainStore().state.headersMetadata.set(merkleBlockFirst.header.hash, {
        height: merkleBlockFirstHeight,
        time: 99999999
      })

      historicalStream.sendMerkleBlock(merkleBlockFirst);

      /** Wait for transactions metadata */
      await waitOneTick();
      await wallet.storage.saveState();

      /**
       * Ensure that chain items for fundingTx have been propagated
       * alongside with the lastSyncedBlockHeight
       */
      storage = storageAdapterMock.getItem(`wallet_${wallet.walletId}`);
      chainStoreState = storage.chains[wallet.network];
      expect(chainStoreState.transactions[fundingTx.hash]).to.exist
      expect(chainStoreState.txMetadata[fundingTx.hash]).to.exist
      expect(chainStoreState.lastSyncedBlockHeight).to.equal(10)

      /** End historical sync */
      historicalStream.end();
      await waitOneTick();

      /**
       * Ensure that reorg safe height (chain height - 6) is set as last known block
       * after historical sync is finished
       */
      await wallet.storage.saveState();
      storage = storageAdapterMock.getItem(`wallet_${wallet.walletId}`)
      chainStoreState = storage.chains[wallet.network];
      expect(chainStoreState.lastSyncedBlockHeight).to.equal(36)

      /** Start continuous sync */
      await txSyncWorker.execute()
      await waitOneTick();

      /** Broadcast transaction from the wallet */
      const sendTx = account.createTransaction({
        recipient: new PrivateKey().toAddress(),
        satoshis: 1000
      });
      account.broadcastTransaction(sendTx)
      await waitOneTick();

      continuousStream.sendTransactions([sendTx]);

      wallet.storage.getDefaultChainStore().state.chainHeight = 43;
      const merkleBlockSecond = mockMerkleBlock([sendTx.hash], merkleBlockFirst.header);
      allMerkleBlocks.push(merkleBlockSecond);
      const merkleBlockSecondHeight = 43;
      wallet.storage.getDefaultChainStore().state.headersMetadata.set(merkleBlockSecond.header.hash, {
        height: merkleBlockSecondHeight,
        time: 99999999
      })

      continuousStream.sendMerkleBlock(merkleBlockSecond);

      await waitOneTick();

      /**
       * Ensure that reorg safe height (chain height - 6) is set as last known block height
       * and sent transaction hasn't been saved because it's still not reorg-safe
       * */
      await wallet.storage.saveState();
      storage = storageAdapterMock.getItem(`wallet_${wallet.walletId}`)
      chainStoreState = storage.chains[wallet.network];
      expect(Object.keys(chainStoreState.transactions)).to.have.lengthOf(1)
      expect(Object.keys(chainStoreState.txMetadata)).to.have.lengthOf(1)
      expect(chainStoreState.lastSyncedBlockHeight).to.equal(37)

      /**
       * Emit one more BLOCKHEIGHT_CHANGE event to ensure that previously considered
       * reorg unsafe items were saved
       */
      wallet.storage.getDefaultChainStore().state.chainHeight = 50;
      const merkleBlockThird = mockMerkleBlock([], merkleBlockSecond.header);
      allMerkleBlocks.push(merkleBlockThird);
      const merkleBlockThirdHeight = 50;
      wallet.storage.getDefaultChainStore().state.headersMetadata.set(merkleBlockThird.header.hash, {
        height: merkleBlockThirdHeight,
        time: 99999999
      })

      continuousStream.sendMerkleBlock(merkleBlockThird);

      await waitOneTick();

      await wallet.storage.saveState();
      storage = storageAdapterMock.getItem(`wallet_${wallet.walletId}`)
      chainStoreState = storage.chains[wallet.network];

      /**
       * Ensure that storage have been updated with the latest
       * transactions and relevant chain data which now considered reorg safe
       */
      expect(Object.keys(chainStoreState.transactions)).to.have.lengthOf(2)
      expect(Object.keys(chainStoreState.txMetadata)).to.have.lengthOf(2)
      expect(chainStoreState.lastSyncedBlockHeight).to.equal(44)

      /** Update chain height */
      bestBlockHeight = 52;
    })

    /**
     * In this scenario we have a wallet that picks part of the data from the storage
     * and then sends a new transaction to the network
     */
    it('should ensure synchronization from last known block for wallet with storage', async ()  => {
      /** Initialize account */
      const account = await wallet.getAccount();

      const walletStore = account.storage.getWalletStore(wallet.walletId);
      const chainStore = account.storage.getChainStore(wallet.network);

      /** Ensure that storage contains transaction and relevant chain data */
      expect(chainStore.state.transactions.size).to.equal(2);
      expect(chainStore.state.lastSyncedBlockHeight).to.equal(44)

      /** Start transactions sync plugin */
      txSyncWorker.onStart();
      await waitOneTick();

      /** End historical sync */
      historicalStream.end();
      await waitOneTick();

      /** Ensure that reorg-safe block set as last known block */
      await wallet.storage.saveState();
      let storage = storageAdapterMock.getItem(`wallet_${wallet.walletId}`)
      let chainStoreState = storage.chains[wallet.network];
      expect(chainStoreState.lastSyncedBlockHeight).to.equal(46)

      /** Start continuous sync */
      await txSyncWorker.execute()
      await waitOneTick();

      /** Broadcast transaction from the wallet */
      const sendTx = account.createTransaction({
        recipient: new PrivateKey().toAddress(),
        satoshis: 1000
      });
      account.broadcastTransaction(sendTx)

      continuousStream.sendTransactions([sendTx]);

      wallet.storage.getDefaultChainStore().state.chainHeight = 52;

      const merkleBlockFourth = mockMerkleBlock([sendTx.hash], allMerkleBlocks[allMerkleBlocks.length - 1].header);
      allMerkleBlocks.push(merkleBlockFourth);
      const merkleBlockThirdHeight = 52;
      wallet.storage.getDefaultChainStore().state.headersMetadata.set(merkleBlockFourth.header.hash, {
        height: merkleBlockThirdHeight,
        time: 99999999
      })

      continuousStream.sendMerkleBlock(merkleBlockFourth);

      /** Wait for sendTx metadata arrives to the storage */
      await waitOneTick();

      /**
       * Ensure that storage still in reorg-safe state
       */
      await wallet.storage.saveState();
      storage = storageAdapterMock.getItem(`wallet_${wallet.walletId}`)
      chainStoreState = storage.chains[wallet.network]

      expect(Object.keys(chainStoreState.transactions)).to.have.lengthOf(2)
      expect(Object.keys(chainStoreState.txMetadata)).to.have.lengthOf(2)
      expect(chainStoreState.lastSyncedBlockHeight).to.equal(46)

      wallet.storage.getDefaultChainStore().state.chainHeight = 59;
      const merkleBlockFifth = mockMerkleBlock([], merkleBlockFourth.header);
      allMerkleBlocks.push(merkleBlockFifth);
      const merkleBlockFifthHeight = 59;
      wallet.storage.getDefaultChainStore().state.headersMetadata.set(merkleBlockFifth.header.hash, {
        height: merkleBlockFifthHeight,
        time: 99999999
      })

      continuousStream.sendMerkleBlock(merkleBlockFifth);

      await waitOneTick();

      await wallet.storage.saveState();
      storage = storageAdapterMock.getItem(`wallet_${wallet.walletId}`)
      chainStoreState = storage.chains[wallet.network]

      /**
       * Ensure that storage have been updated with the latest
       * transactions and relevant chain data which now considered reorg safe
       */
      expect(Object.keys(chainStoreState.transactions)).to.have.lengthOf(3)
      expect(Object.keys(chainStoreState.txMetadata)).to.have.lengthOf(3)
      expect(chainStoreState.lastSyncedBlockHeight).to.equal(53)
    })
  })
})
