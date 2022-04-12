const {
  HDPrivateKey,
  Transaction,
  BlockHeader,
  PrivateKey
} = require('@dashevo/dashcore-lib');

const { expect } = require('chai');

const { Wallet, EVENTS } = require("../../../src");
const TransactionSyncStreamWorker = require("../../../src/plugins/Workers/TransactionSyncStreamWorker/TransactionSyncStreamWorker");
const ChainPlugin = require("../../../src/plugins/Plugins/ChainPlugin");
const LocalForageAdapterMock = require("../../../src/test/mocks/LocalForageAdapterMock");
const createAndAttachTransportMocksToWallet = require("../../../src/test/mocks/createAndAttachTransportMocksToWallet");
const {sleepOneTick} = require("../../../src/test/utils");

describe('Wallet', () => {
  describe('Storage', () => {
    let wallet;
    let txStreamMock;
    let txStreamWorker;
    let chainPlugin;
    let transportMock;
    let storageAdapterMock = new LocalForageAdapterMock();;

    beforeEach(async function() {
      const testHDKey = "xprv9s21ZrQH143K4PgfRZPuYjYUWRZkGfEPuWTEUESMoEZLC274ntC4G49qxgZJEPgmujsmY52eVggtwZgJPrWTMXmbYgqDVySWg46XzbGXrSZ";
      txStreamWorker = new TransactionSyncStreamWorker({ executeOnStart: false });
      chainPlugin = new ChainPlugin({ executeOnStart: false });

      wallet = new Wallet({
        offlineMode: true,
        plugins: [chainPlugin, txStreamWorker],
        allowSensitiveOperations: true,
        HDPrivateKey: new HDPrivateKey(testHDKey),
        adapter: storageAdapterMock,
        network: 'livenet'
      });

      ({ txStreamMock, transportMock } = await createAndAttachTransportMocksToWallet(wallet, this.sinonSandbox));

      await chainPlugin.onStart()
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
        blockHeaders: [
          new BlockHeader({
            version: 1,
            prevHash: '0000000000000000000000000000000000000000000000000000000000000000',
            merkleRoot: '0000000000000000000000000000000000000000000000000000000000000000',
            time: Date.now() / 1000,
            bits: 0,
            nonce: 0,
          }),
          new BlockHeader({
            version: 1,
            prevHash: '0000000000000000000000000000000000000000000000000000000000000001',
            merkleRoot: '0000000000000000000000000000000000000000000000000000000000000000',
            time: Date.now() / 1000,
            bits: 1,
            nonce: 1,
          })
        ],
        metadata: {}
      }

      transportMock.getBestBlockHeight.returns(42);
      transportMock.getTransaction.callsFake(async (hash) => scenario.metadata[hash])
      transportMock.getBlockHeaderByHash.callsFake(async hash => scenario.blockHeaders.find(header => header.hash === hash))

      Object.assign(scenario.metadata, {
        [scenario.transactions.fundingTx.hash]: {
          transaction: scenario.transactions.fundingTx,
          height: 10,
          blockHash: scenario.blockHeaders[0].hash
        }
      })

      /** Start transactions sync plugin */
      txStreamWorker.onStart();
      await sleepOneTick();

      /** Ensure proper transport arguments */
      expect(transportMock.subscribeToTransactionsWithProofs.firstCall.args[1])
        .to.deep.equal({ fromBlockHeight: 1, count: 41 });

      /** Send funding transaction to the wallet */
      const { fundingTx } = scenario.transactions;
      txStreamMock.sendTransactions([fundingTx]);
      await wallet.storage.saveState();

      /** Ensure that storage has no items for transactions without the metadata */
      let chainStoreState = storageAdapterMock.getItem('chains')[wallet.network];
      let walletStoreState = storageAdapterMock.getItem('wallets')[wallet.walletId]
      expect(chainStoreState.transactions).to.be.empty;
      expect(chainStoreState.txMetadata).to.be.empty;
      expect(chainStoreState.blockHeaders).to.be.empty;
      expect(walletStoreState.lastKnownBlock.height).to.equal(-1)

      /** Wait for transactions metadata */
      await sleepOneTick();

      /**
       * Simulate block height change to ensure that the value is not
       * populated as WalletStore.state.lastKnownBlock during the historical sync
       */
      transportMock.emit(EVENTS.BLOCKHEIGHT_CHANGED, { payload: 43 })

      /** Wait for blockheight propagates to the storage */
      await sleepOneTick();

      await wallet.storage.saveState();

      /**
       * Ensure that chain items for fundingTx have been propagated
       * alongside with the lastKnownBlock
       */
      chainStoreState = storageAdapterMock.getItem('chains')[wallet.network];
      walletStoreState = storageAdapterMock.getItem('wallets')[wallet.walletId]
      expect(chainStoreState.transactions[fundingTx.hash]).to.exist;
      expect(chainStoreState.txMetadata[fundingTx.hash]).to.exist
      expect(chainStoreState.blockHeaders[scenario.blockHeaders[0].hash]).to.exist;
      expect(walletStoreState.lastKnownBlock.height).to.equal(10)

      /** End historical sync */
      txStreamMock.finish();
      await sleepOneTick();

      /** Ensure that best block is set as last known block */
      await wallet.storage.saveState();
      walletStoreState = storageAdapterMock.getItem('wallets')[wallet.walletId]
      expect(walletStoreState.lastKnownBlock.height).to.equal(42)

      /** Start continuous sync */
      txStreamWorker.execute()
      await sleepOneTick();

      /** Ensure proper transport arguments */
      expect(transportMock.subscribeToTransactionsWithProofs.lastCall.args[1])
        .to.deep.equal({ fromBlockHeight: 42, count: 0 });

      /** Broadcast transaction from the wallet */
      const sendTx = account.createTransaction({
        recipient: new PrivateKey().toAddress(),
        satoshis: 1000
      });
      await account.broadcastTransaction(sendTx)

      txStreamMock.sendTransactions([sendTx]);
      Object.assign(scenario.metadata, {
        [sendTx.hash]: {
          transaction: sendTx,
          height: 43,
          blockHash: scenario.blockHeaders[1].hash
        }
      })

      /** Wait for sendTx metadata arrives to the storage */
      await sleepOneTick();

      /** Ensure that sendTx height is set as last known block height */
      await wallet.storage.saveState();
      walletStoreState = storageAdapterMock.getItem('wallets')[wallet.walletId]
      expect(walletStoreState.lastKnownBlock.height).to.equal(43)

      /**
       * Emit one more BLOCKHEIGHT_CHANGE event to ensure that it's value will be saved
       * as WalletStore.state.lastKnownBlock
       */
      transportMock.emit(EVENTS.BLOCKHEIGHT_CHANGED, { payload: 44 })
      await sleepOneTick();

      await wallet.storage.saveState();
      chainStoreState = storageAdapterMock.getItem('chains')[wallet.network];
      walletStoreState = storageAdapterMock.getItem('wallets')[wallet.walletId]

      /**
       * Ensure that storage have been updated with the latest
       * transaction and relevant chain data
       */
      expect(Object.keys(chainStoreState.transactions)).to.have.lengthOf(2)
      expect(Object.keys(chainStoreState.txMetadata)).to.have.lengthOf(2)
      expect(Object.keys(chainStoreState.blockHeaders)).to.have.lengthOf(2)
      expect(walletStoreState.lastKnownBlock.height).to.equal(44)
    })

    /**
     * In this scenario we have a wallet that picks part of the data from the storage
     * and then sends a new transaction to the network
     */
    it('should ensure synchronization from last known block for wallet with storage', async ()  => {
      const scenario = {
        blockHeaders: [
          new BlockHeader({
            version: 1,
            prevHash: '0000000000000000000000000000000000000000000000000000000000000002',
            merkleRoot: '0000000000000000000000000000000000000000000000000000000000000000',
            time: Date.now() / 1000,
            bits: 0,
            nonce: 0,
          }),
        ],
        metadata: {}
      }

      transportMock.getTransaction.callsFake(async (hash) => scenario.metadata[hash])
      transportMock.getBestBlockHeight.returns(50);
      transportMock.getBlockHeaderByHash
        .callsFake(async hash => scenario.blockHeaders.find(header => header.hash === hash))

      /** Initialize account */
      const account = await wallet.getAccount();

      const walletStore = account.storage.getWalletStore(wallet.walletId);
      const chainStore = account.storage.getChainStore(wallet.network);

      /** Ensure that storage contains transaction and relevant chain data */
      expect(chainStore.state.transactions.size).to.equal(2);
      expect(chainStore.state.blockHeaders.size).to.equal(2)
      expect(walletStore.state.lastKnownBlock.height).to.equal(44)

      /** Start transactions sync plugin */
      txStreamWorker.onStart();
      await sleepOneTick();

      /** Ensure that historical synchronization starts from last known block */
      expect(transportMock.subscribeToTransactionsWithProofs.lastCall.args[1])
        .to.deep.equal({ fromBlockHeight: 44, count: 6 });

      /** End historical sync */
      txStreamMock.finish();
      await sleepOneTick();

      /** Ensure that best block is set as last known block */
      await wallet.storage.saveState();
      let walletStoreState = storageAdapterMock.getItem('wallets')[wallet.walletId]
      expect(walletStoreState.lastKnownBlock.height).to.equal(50)

      /** Start continuous sync */
      txStreamWorker.execute()
      await sleepOneTick();

      /** Ensure proper transport arguments */
      expect(transportMock.subscribeToTransactionsWithProofs.lastCall.args[1])
        .to.deep.equal({ fromBlockHeight: 50, count: 0 });

      /** Broadcast transaction from the wallet */
      const sendTx = account.createTransaction({
        recipient: new PrivateKey().toAddress(),
        satoshis: 1000
      });
      await account.broadcastTransaction(sendTx)

      txStreamMock.sendTransactions([sendTx]);
      Object.assign(scenario.metadata, {
        [sendTx.hash]: {
          transaction: sendTx,
          height: 51,
          blockHash: scenario.blockHeaders[0].hash
        }
      })

      /** Wait for sendTx metadata arrives to the storage */
      await sleepOneTick();

      /**
       * Ensure that storage have been updated with the latest
       * transaction and relevant chain data
       */
      await wallet.storage.saveState();
      walletStoreState = storageAdapterMock.getItem('wallets')[wallet.walletId]
      const chainStoreState = storageAdapterMock.getItem('chains')[wallet.network];

      expect(Object.keys(chainStoreState.transactions)).to.have.lengthOf(3)
      expect(Object.keys(chainStoreState.txMetadata)).to.have.lengthOf(3)
      expect(Object.keys(chainStoreState.blockHeaders)).to.have.lengthOf(3)
      expect(walletStoreState.lastKnownBlock.height).to.equal(51)
    })
  })
})
