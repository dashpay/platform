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
const TxStreamMock = require("../../../src/test/mocks/TxStreamMock");
const {sleepOneTick} = require("../../../src/test/utils");
const {txFilterStream} = require("../../../../dapi/lib/config");


describe('Wallet', () => {
  describe('Storage', () => {
    let wallet;
    let txStreamMock;
    let txStreamWorker;
    let chainPlugin;
    let transportMock;
    // let tx

    beforeEach(async function() {
      const testHDKey = "xprv9s21ZrQH143K4PgfRZPuYjYUWRZkGfEPuWTEUESMoEZLC274ntC4G49qxgZJEPgmujsmY52eVggtwZgJPrWTMXmbYgqDVySWg46XzbGXrSZ";
      txStreamWorker = new TransactionSyncStreamWorker({ executeOnStart: false });
      chainPlugin = new ChainPlugin({ executeOnStart: false });

      const storageAdapterMock = new LocalForageAdapterMock();

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
      //
      // const account = await wallet.getAccount();

      // console.log(wallet.storage.getWalletStore(wallet.walletId).exportState())
      // console.log(wallet.storage.getChainStore('livenet').exportState())
    })

    /**
     * In this scenario we have a fresh wallet that receives a funding transaction
     * and sends a transaction in his own
     */
    it.only('should fill the storage for a fresh wallet', async function() {
      const account = await wallet.getAccount();
      const { address: addressToFund } = account.getUnusedAddress();
      const chainStore = wallet.storage.getChainStore(wallet.network);
      const walletStore = wallet.storage.getWalletStore(wallet.walletId);

      /**
       * Define a scenario
       */
      const scenario = {
        transactions: {
          fundingTx: new Transaction().to(addressToFund, 10000),
        },
        blockHeaders: [
          new BlockHeader({
            version: 1,
            prevHash: '0x0000000000000000000000000000000000000000000000000000000000000000',
            merkleRoot: '0x0000000000000000000000000000000000000000000000000000000000000000',
            time: Date.now() / 1000,
            bits: 0,
            nonce: 0,
          }),
          new BlockHeader({
            version: 1,
            prevHash: '0x0000000000000000000000000000000000000000000000000000000000000001',
            merkleRoot: '0x0000000000000000000000000000000000000000000000000000000000000000',
            time: Date.now() / 1000,
            bits: 1,
            nonce: 1,
          })
        ],
        metadata: {}
      }

      transportMock.getTransaction.callsFake(async (hash) => scenario.metadata[hash])
      transportMock.getBlockHeaderByHash.callsFake(async hash => scenario.blockHeaders.find(header => header.hash === hash))

      Object.assign(scenario.metadata, {
        [scenario.transactions.fundingTx.hash]: {
          transaction: scenario.transactions.fundingTx,
          height: 10,
          blockHash: scenario.blockHeaders[0].hash
        }
      })

      /**
       * Start transactions sync plugin
       */
      txStreamWorker.onStart();

      await sleepOneTick();

      /**
       * Send funding transaction to the wallet
       */
      const { fundingTx } = scenario.transactions;
      txStreamMock.sendTransactions([fundingTx]);

      // Storage should not have transaction exported if it has no metadata yet
      let chainStoreState = chainStore.exportState();
      let walletStoreState = walletStore.exportState();
      expect(chainStoreState.transactions).to.be.empty;
      expect(chainStoreState.txMetadata).to.be.empty;
      expect(chainStoreState.blockHeaders).to.be.empty;
      expect(walletStoreState.lastKnownBlock.height).to.equal(1)

      /**
       * Simulate block height change to ensure that the value is not
       * populated as WalletStore.state.lastKnownBlock during the historical sync
       */
      transportMock.emit(EVENTS.BLOCKHEIGHT_CHANGED, { payload: 43 })

      /**
       * End historical sync
       */
      txStreamMock.finish();

      /**
       * Start continuous sync
       */
      txStreamWorker.execute()

      await sleepOneTick();

      chainStoreState = chainStore.exportState();
      walletStoreState = walletStore.exportState();
      expect(chainStoreState.transactions[fundingTx.hash]).to.exist;
      expect(chainStoreState.txMetadata[fundingTx.hash]).to.exist
      expect(chainStoreState.blockHeaders[scenario.blockHeaders[0].hash]).to.exist;
      expect(walletStoreState.lastKnownBlock.height).to.equal(10)

      /**
       * Send transaction from the wallet
       */
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

      await sleepOneTick();

      /**
       * Emit one more BLOCKHEIGHT_CHANGE event to ensure that it's value will be saved
       * as WalletStore.state.lastKnownBlock
       */
      transportMock.emit(EVENTS.BLOCKHEIGHT_CHANGED, { payload: 44 })

      await sleepOneTick();

      chainStoreState = chainStore.exportState();
      walletStoreState = walletStore.exportState();
      // Storage should contain both transactions, their metadata, and block headers
      expect(Object.keys(chainStoreState.transactions)).to.have.lengthOf(2)
      expect(Object.keys(chainStoreState.txMetadata)).to.have.lengthOf(2)
      expect(Object.keys(chainStoreState.blockHeaders)).to.have.lengthOf(2)
      expect(walletStoreState.lastKnownBlock.height).to.equal(44)


      // Ensure:
      // - transactions without metadata are not saved
      // - last known block equals to chain height
      // - make sure storage integrity is intact
    })

    it('should fill storage for imported wallet', ()  => {
      // Import mnemonic to the wallet
      //
      // Wait for historical sync completes
      //
      // Ensure:
      // - storage integrity intact
      // - last known block equals to the chain height on the startup
    })

    it('should fill storage for imported wallet and emit couple of transactions', () => {
      // Import mnemomic to the wallet
      //
      // Wait for historical sync completes
      //
      // Start continuous sync
      //
      // Ensure:
      // - storage integrity
      // - Calls to corresponding save state with the required arguments:
      //    - During the historical sync, only with TX metadata values
      //    - During the continuous sync, with chain height
    })

    it('should do sync from the last known block', () => {
      // Have a storage with data prior to the last known block
      // Ensure it's integrity
      //
      // Start downloading new blocks
      //
      // Wait for finish
      //
      // Ensure that integrity is intact
    })
  })
})
