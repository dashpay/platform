const {
  HDPrivateKey,
  Transaction,
  PrivateKey
} = require('@dashevo/dashcore-lib');

const { expect } = require('chai');

const { Wallet, EVENTS } = require("../../../src");
const TransactionSyncStreamWorker = require("../../../src/plugins/Workers/TransactionSyncStreamWorker/TransactionSyncStreamWorker");
const ChainPlugin = require("../../../src/plugins/Plugins/ChainPlugin");
const LocalForageAdapterMock = require("../../../src/test/mocks/LocalForageAdapterMock");
const createAndAttachTransportMocksToWallet = require("../../../src/test/mocks/createAndAttachTransportMocksToWallet");
const {waitOneTick} = require("../../../src/test/utils");
const mockMerkleBlock = require("../../../src/test/mocks/mockMerkleBlock");

describe('Wallet', () => {
  // TODO(spv): write test that ensures that storage getting wiped after removing skipSynchronizationBeofreHeight flag
  describe('Storage', () => {
    let wallet;
    let txStreamMock;
    let txStreamWorker;
    let chainPlugin;
    let transportMock;
    let bestBlockHeight = 42;
    let storageAdapterMock = new LocalForageAdapterMock();

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

      ({ txStreamMock, transportMock } = await createAndAttachTransportMocksToWallet(wallet, this.sinon));

      transportMock.getStatus.returns({
        chain: { blocksCount: bestBlockHeight },
        network: { fee: 237 }
      })

      transportMock.sendTransaction.callsFake((tx) => {
        txStreamMock.sendTransactions([new Transaction(tx)])
      })

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
      }

      /** Start transactions sync plugin */
      txStreamWorker.onStart();
      await waitOneTick();

      /** Ensure proper transport arguments */
      expect(transportMock.subscribeToTransactionsWithProofs.firstCall.args[1])
        .to.deep.equal({ fromBlockHeight: 1, count: 41 });

      /** Send first funding transaction to the wallet */
      const { fundingTx } = scenario.transactions;
      txStreamMock.sendTransactions([fundingTx]);

      await waitOneTick();

      await wallet.storage.saveState();

      /** Ensure that storage has no items for transactions without the metadata */
      let storage = storageAdapterMock.getItem(`wallet_${wallet.walletId}`)
      let chainStoreState = storage.chains[wallet.network].chain;
      let walletStoreState = storage.chains[wallet.network].wallet;
      expect(chainStoreState.transactions).to.be.empty;
      expect(chainStoreState.txMetadata).to.be.empty;
      // -6 to ensure reorg safe saving procedure
      expect(walletStoreState.lastKnownBlock.height).to.equal(0)

      const merkleBlock = mockMerkleBlock([fundingTx.hash]);
      const merkleBlockHeight = 10;
      wallet.storage.getDefaultChainStore().state.headersMetadata.set(merkleBlock.header.hash, {
        height: merkleBlockHeight,
        time: 99999999
      })

      txStreamMock.sendMerkleBlock(merkleBlock);

      /** Wait for transactions metadata */
      await waitOneTick();
      await wallet.storage.saveState();

      /**
       * Ensure that chain items for fundingTx have been propagated
       * alongside with the lastKnownBlock
       */
      storage = storageAdapterMock.getItem(`wallet_${wallet.walletId}`)
      chainStoreState = storage.chains[wallet.network].chain;
      walletStoreState = storage.chains[wallet.network].wallet;
      expect(chainStoreState.transactions[fundingTx.hash]).to.exist
      expect(chainStoreState.txMetadata[fundingTx.hash]).to.exist
      expect(walletStoreState.lastKnownBlock.height).to.equal(10)

      /** End historical sync */
      txStreamMock.finish();
      await waitOneTick();

      /**
       * Ensure that reorg safe height (chain height - 6) is set as last known block
       * after historical sync is finished
       */
      await wallet.storage.saveState();
      storage = storageAdapterMock.getItem(`wallet_${wallet.walletId}`)
      walletStoreState = storage.chains[wallet.network].wallet;
      expect(walletStoreState.lastKnownBlock.height).to.equal(36)

      /** Start continuous sync */
      await txStreamWorker.execute()
      await waitOneTick();

      /** Ensure proper transport arguments */
      expect(transportMock.subscribeToTransactionsWithProofs.lastCall.args[1])
        .to.deep.equal({ fromBlockHeight: 42, count: 0 });

      /** Broadcast transaction from the wallet */
      const sendTx = account.createTransaction({
        recipient: new PrivateKey().toAddress(),
        satoshis: 1000
      });
      await account.broadcastTransaction(sendTx)

      wallet.storage.getDefaultChainStore().state.blockHeight = 43;
      account.emit(EVENTS.BLOCK, { hash: '1111', transactions: [sendTx]}, 43)

      await waitOneTick();

      /**
       * Ensure that reorg safe height (chain height - 6) is set as last known block height
       * and sent transaction hasn't been saved because it's still not reorg-safe
       * */
      await wallet.storage.saveState();
      storage = storageAdapterMock.getItem(`wallet_${wallet.walletId}`)
      walletStoreState = storage.chains[wallet.network].wallet;
      expect(Object.keys(chainStoreState.transactions)).to.have.lengthOf(1)
      expect(Object.keys(chainStoreState.txMetadata)).to.have.lengthOf(1)
      expect(walletStoreState.lastKnownBlock.height).to.equal(37)

      /**
       * Emit one more BLOCKHEIGHT_CHANGE event to ensure that previously considered
       * reorg unsafe items were saved
       */
      wallet.storage.getDefaultChainStore().state.blockHeight = 50;
      account.emit(EVENTS.BLOCK, { hash: '1112', transactions: []}, 50)
      await waitOneTick();

      await wallet.storage.saveState();
      storage = storageAdapterMock.getItem(`wallet_${wallet.walletId}`)
      chainStoreState = storage.chains[wallet.network].chain;
      walletStoreState = storage.chains[wallet.network].wallet;

      /**
       * Ensure that storage have been updated with the latest
       * transactions and relevant chain data which now considered reorg safe
       */
      expect(Object.keys(chainStoreState.transactions)).to.have.lengthOf(2)
      expect(Object.keys(chainStoreState.txMetadata)).to.have.lengthOf(2)
      expect(walletStoreState.lastKnownBlock.height).to.equal(44)

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
      expect(walletStore.state.lastKnownBlock.height).to.equal(44)

      /** Start transactions sync plugin */
      txStreamWorker.onStart();
      await waitOneTick();

      /** Ensure that historical synchronization starts from last known block */
      expect(transportMock.subscribeToTransactionsWithProofs.lastCall.args[1])
        .to.deep.equal({ fromBlockHeight: 44, count: 8 });

      /** End historical sync */
      txStreamMock.finish();
      await waitOneTick();

      /** Ensure that reorg-safe block set as last known block */
      await wallet.storage.saveState();
      let storage = storageAdapterMock.getItem(`wallet_${wallet.walletId}`)
      let walletStoreState = storage.chains[wallet.network].wallet
      expect(walletStoreState.lastKnownBlock.height).to.equal(46)

      /** Start continuous sync */
      await txStreamWorker.execute()
      await waitOneTick();

      /** Ensure proper transport arguments */
      expect(transportMock.subscribeToTransactionsWithProofs.lastCall.args[1])
        .to.deep.equal({ fromBlockHeight: 52, count: 0 });

      /** Broadcast transaction from the wallet */
      const sendTx = account.createTransaction({
        recipient: new PrivateKey().toAddress(),
        satoshis: 1000
      });
      await account.broadcastTransaction(sendTx)

      wallet.storage.getDefaultChainStore().state.blockHeight = 52;
      account.emit(EVENTS.BLOCK, { hash: '1111', transactions: [sendTx]}, 52);

      /** Wait for sendTx metadata arrives to the storage */
      await waitOneTick();

      /**
       * Ensure that storage still in reorg-safe state
       */
      await wallet.storage.saveState();
      storage = storageAdapterMock.getItem(`wallet_${wallet.walletId}`)
      let chainStoreState = storage.chains[wallet.network].chain
      walletStoreState = storage.chains[wallet.network].wallet

      expect(Object.keys(chainStoreState.transactions)).to.have.lengthOf(2)
      expect(Object.keys(chainStoreState.txMetadata)).to.have.lengthOf(2)
      expect(walletStoreState.lastKnownBlock.height).to.equal(46)


      /**
       * Emit one more BLOCKHEIGHT_CHANGE event to ensure that previously considered
       * reorg unsafe items were saved
       */
      wallet.storage.getDefaultChainStore().state.blockHeight = 59;
      account.emit(EVENTS.BLOCK, { hash: '1112', transactions: []}, 59);
      await waitOneTick();

      await wallet.storage.saveState();
      storage = storageAdapterMock.getItem(`wallet_${wallet.walletId}`)
      chainStoreState = storage.chains[wallet.network].chain
      walletStoreState = storage.chains[wallet.network].wallet

      /**
       * Ensure that storage have been updated with the latest
       * transactions and relevant chain data which now considered reorg safe
       */
      expect(Object.keys(chainStoreState.transactions)).to.have.lengthOf(3)
      expect(Object.keys(chainStoreState.txMetadata)).to.have.lengthOf(3)
      expect(walletStoreState.lastKnownBlock.height).to.equal(53)
    })
  })
})
