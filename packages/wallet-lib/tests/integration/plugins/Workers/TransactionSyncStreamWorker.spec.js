const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');

const {
  HDPrivateKey,
  Transaction,
  BlockHeader,
  MerkleBlock,
  InstantLock
} = require('@dashevo/dashcore-lib');

const TransactionSyncStreamWorker = require('../../../../src/plugins/Workers/TransactionSyncStreamWorker/TransactionSyncStreamWorker');

const TxStreamDataResponseMock = require('../../../../src/test/mocks/TxStreamDataResponseMock');
const TxStreamMock = require('../../../../src/test/mocks/TxStreamMock');

const createAndAttachTransportMocksToWallet = require('../../../../src/test/mocks/createAndAttachTransportMocksToWallet')

const { Wallet } = require('../../../../src');

const blockHeaderFixture = '00000020e2bddfb998d7be4cc4c6b126f04d6e4bd201687523ded527987431707e0200005520320b4e263bec33e08944656f7ce17efbc2c60caab7c8ed8a73d413d02d3a169d555ecdd6021e56d000000203000500010000000000000000000000000000000000000000000000000000000000000000ffffffff050219250102ffffffff0240c3609a010000001976a914ecfd5aaebcbb8f4791e716e188b20d4f0183265c88ac40c3609a010000001976a914ecfd5aaebcbb8f4791e716e188b20d4f0183265c88ac0000000046020019250000476416132511031b71167f4bb7658eab5c3957d79636767f83e0e18e2b9ed7f8000000000000000000000000000000000000000000000000000000000000000003000600000000000000fd4901010019250000010001d02e9ee1b14c022ad6895450f3375a8e9a87f214912d4332fa997996d2000000320000000000000032000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000';
chai.use(chaiAsPromised);
const { expect } = chai;

function wait(ms) {
  return new Promise((res) => setTimeout(res, ms));
}

describe('TransactionSyncStreamWorker', function suite() {
  this.timeout(60000);
  let worker;
  let storage;
  let walletId;
  let wallet;
  let account;
  let txStreamMock;
  let address;
  let addressAtIndex19;
  let testHDKey;
  let merkleBlockMock;
  let transportMock;

  beforeEach(async function beforeEach() {
    testHDKey = "xprv9s21ZrQH143K4PgfRZPuYjYUWRZkGfEPuWTEUESMoEZLC274ntC4G49qxgZJEPgmujsmY52eVggtwZgJPrWTMXmbYgqDVySWg46XzbGXrSZ";
    merkleBlockMock = new MerkleBlock(Buffer.from([0,0,0,32,61,11,102,108,38,155,164,49,91,246,141,178,126,155,13,118,248,83,250,15,206,21,102,65,104,183,243,167,235,167,60,113,140,110,120,87,208,191,240,19,212,100,228,121,192,125,143,44,226,9,95,98,51,25,139,172,175,27,205,201,158,85,37,8,72,52,36,95,255,255,127,32,2,0,0,0,1,0,0,0,1,140,110,120,87,208,191,240,19,212,100,228,121,192,125,143,44,226,9,95,98,51,25,139,172,175,27,205,201,158,85,37,8,1,1]));

    testHDKey = new HDPrivateKey(testHDKey).toString();

    // Override default value of executeOnStart to prevent worker from starting
    worker = new TransactionSyncStreamWorker({ executeOnStart: false });

    // This is a full instance of wallet with a mocked transport
    wallet = new Wallet({
      offlineMode: true,
      plugins: [worker],
      allowSensitiveOperations: true,
      HDPrivateKey: new HDPrivateKey(testHDKey),
    });

    ({ txStreamMock, transportMock } = await createAndAttachTransportMocksToWallet(wallet, this.sinonSandbox));

    transportMock.getBlockHeaderByHash
        .returns(BlockHeader.fromString(blockHeaderFixture));

    account = await wallet.getAccount();

    storage = account.storage;
    walletId = Object.keys(storage.store.wallets)[0];

    address = account.getAddress(0).address;
    addressAtIndex19 = account.getAddress(19).address;
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

      account.transport.getBestBlockHeight
        .returns(bestBlockHeight);

      setTimeout(async () => {
        try {
          expect(worker.stream).is.not.null;

          for (let i = lastSavedBlockHeight; i <= bestBlockHeight; i++) {
            const transaction = new Transaction().to(address, i);
            account.transport.getTransaction
                .returns({
                  transaction:transaction,
                  blockHash: Buffer.from('4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176', 'hex'),
                  height: 42,
                  confirmations: 10,
                  isInstantLocked: true,
                  isChainLocked: false,
                });
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

      account.transport.getBestBlockHeight
        .returns(bestBlockHeight);

      setTimeout(async () => {
        try {
          expect(worker.stream).is.not.null;

          let transaction = new Transaction().to(addressAtIndex19, 10000);
          account.transport.getTransaction
              .returns({
                transaction:transaction,
                blockHash: Buffer.from('4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176', 'hex'),
                height: 42,
                confirmations: 10,
                isInstantLocked: true,
                isChainLocked: false,
              });
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

          transaction = new Transaction().to(account.getAddress(10).address, 10000);
          account.transport.getTransaction
              .returns({
                transaction:transaction,
                blockHash: Buffer.from('4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176', 'hex'),
                height: 42,
                confirmations: 10,
                isInstantLocked: true,
                isChainLocked: false,
              });
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
      expect(account.transport.subscribeToTransactionsWithProofs.callCount).to.be.equal(3);
      // 20 external and 20 internal
      expect(account.transport.subscribeToTransactionsWithProofs.firstCall.args[0].length).to.be.equal(40);
      expect(account.transport.subscribeToTransactionsWithProofs.firstCall.args[1]).to.be.deep.equal({ fromBlockHeight: 40, count: 2});
      // 20 more of each type, since the last address is used, but the height is the same, since Merkle Block not received yet
      expect(account.transport.subscribeToTransactionsWithProofs.secondCall.args[0].length).to.be.equal(80);
      expect(account.transport.subscribeToTransactionsWithProofs.secondCall.args[1]).to.be.deep.equal({ fromBlockHash: '0000025d24ebe65454bd51a61bab94095a6ad1df996be387e31495f764d8e2d9', count: 2});
      // Block received
      expect(account.transport.subscribeToTransactionsWithProofs.thirdCall.args[0].length).to.be.equal(80);
      expect(account.transport.subscribeToTransactionsWithProofs.thirdCall.args[1]).to.be.deep.equal({ fromBlockHash: '0000025d24ebe65454bd51a61bab94095a6ad1df996be387e31495f764d8e2d9', count: 2});

      expect(worker.stream).to.be.null;
      expect(transactionsInStorage.length).to.be.equal(2);
      expect(transactionsInStorage).to.have.deep.members(expectedTransactions);
    });
    it('should reconnect to the historical stream if stream is closed due to operational GRPC error', async function () {
      const lastSavedBlockHeight = 40;
      const bestBlockHeight = 42;

      worker.setLastSyncedBlockHeight(lastSavedBlockHeight);
      const transactionsSent = [];

      account.transport.getBestBlockHeight
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
      expect(account.transport.subscribeToTransactionsWithProofs.callCount).to.be.equal(2);
      // 20 external and 20 internal
      expect(account.transport.subscribeToTransactionsWithProofs.firstCall.args[0].length).to.be.equal(40);
      expect(account.transport.subscribeToTransactionsWithProofs.firstCall.args[1]).to.be.deep.equal({ fromBlockHeight: 40, count: 2});

      expect(account.transport.subscribeToTransactionsWithProofs.secondCall.args[0].length).to.be.equal(40);
      expect(account.transport.subscribeToTransactionsWithProofs.secondCall.args[1]).to.be.deep.equal({ fromBlockHeight: 40, count: 2});

      expect(worker.stream).to.be.null;
    });

    it('should not reconnect to the historical stream if stream in case of any other error', async function () {
      const lastSavedBlockHeight = 40;
      const bestBlockHeight = 42;

      worker.setLastSyncedBlockHeight(lastSavedBlockHeight);
      const transactionsSent = [];

      account.transport.getBestBlockHeight
          .returns(bestBlockHeight);

      setTimeout(async () => {
        expect(worker.stream).is.not.null;

        txStreamMock.emit(TxStreamMock.EVENTS.error, new Error('Some random error'));
      }, 10);

      await expect(worker.onStart()).to.be.rejectedWith('Some random error');

      const addressesInStorage = storage.store.wallets[walletId].addresses.external;

      expect(Object.keys(addressesInStorage).length).to.be.equal(20);
      // Shouldn't try to reconnect
      expect(account.transport.subscribeToTransactionsWithProofs.callCount).to.be.equal(1);
      // 20 external and 20 internal
      expect(account.transport.subscribeToTransactionsWithProofs.firstCall.args[0].length).to.be.equal(40);
      expect(account.transport.subscribeToTransactionsWithProofs.firstCall.args[1]).to.be.deep.equal({ fromBlockHeight: 40, count: 2});

      expect(worker.stream).to.be.null;
    });
  });

  describe("#execute", () => {
    it('should sync incoming transactions and save it to the storage', async function () {
      const lastSavedBlockHeight = 40;
      const bestBlockHeight = 42;

      worker.setLastSyncedBlockHeight(lastSavedBlockHeight);
      const transactionsSent = [];

      account.transport.getBestBlockHeight
          .returns(bestBlockHeight);

      worker.execute();

      await wait(10);

      try {
        for (let i = lastSavedBlockHeight; i <= bestBlockHeight; i++) {
          const transaction = new Transaction().to(address, i);
          account.transport.getTransaction
              .returns({
                transaction:transaction,
                blockHash: Buffer.from('4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176', 'hex'),
                height: 42,
                confirmations: 10,
                isInstantLocked: true,
                isChainLocked: false,
              });
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

      account.transport.getBestBlockHeight
          .returns(bestBlockHeight);

      worker.execute();

      await wait(10);

      try {
        let transaction = new Transaction().to(addressAtIndex19, 10000);
        account.transport.getTransaction
            .returns({
              transaction:transaction,
              blockHash: Buffer.from('4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176', 'hex'),
              height: 42,
              confirmations: 10,
              isInstantLocked: true,
              isChainLocked: false,
            });
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

        transaction = transaction = new Transaction().to(account.getAddress(10).address, 10000);
        account.transport.getTransaction
            .returns({
              transaction:transaction,
              blockHash: Buffer.from('4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176', 'hex'),
              height: 42,
              confirmations: 10,
              isInstantLocked: true,
              isChainLocked: false,
            });
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
      expect(account.transport.subscribeToTransactionsWithProofs.callCount).to.be.equal(3);
      // 20 external and 20 internal
      expect(account.transport.subscribeToTransactionsWithProofs.firstCall.args[0].length).to.be.equal(40);
      expect(account.transport.subscribeToTransactionsWithProofs.firstCall.args[1]).to.be.deep.equal({ fromBlockHeight: 40, count: 0});
      // 20 more of each type, since the last address is used, but the height is the same, since Merkle Block not received yet
      expect(account.transport.subscribeToTransactionsWithProofs.secondCall.args[0].length).to.be.equal(80);
      expect(account.transport.subscribeToTransactionsWithProofs.secondCall.args[1]).to.be.deep.equal({ fromBlockHash: '0000025d24ebe65454bd51a61bab94095a6ad1df996be387e31495f764d8e2d9', count: 0});
      // Block received
      expect(account.transport.subscribeToTransactionsWithProofs.thirdCall.args[0].length).to.be.equal(80);
      expect(account.transport.subscribeToTransactionsWithProofs.thirdCall.args[1]).to.be.deep.equal({ fromBlockHash: '0000025d24ebe65454bd51a61bab94095a6ad1df996be387e31495f764d8e2d9', count: 0});

      expect(worker.stream).to.be.null;
      expect(transactionsInStorage.length).to.be.equal(2);
      expect(transactionsInStorage).to.have.deep.members(expectedTransactions);
    });

    it('should reconnect to the incoming stream if stream is closed due to operational GRPC error', async function () {
      const lastSavedBlockHeight = 40;
      const bestBlockHeight = 42;

      worker.setLastSyncedBlockHeight(lastSavedBlockHeight);

      account.transport.getBestBlockHeight
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
      expect(account.transport.subscribeToTransactionsWithProofs.callCount).to.be.equal(2);
      // 20 external and 20 internal
      expect(account.transport.subscribeToTransactionsWithProofs.firstCall.args[0].length).to.be.equal(40);
      expect(account.transport.subscribeToTransactionsWithProofs.firstCall.args[1]).to.be.deep.equal({ fromBlockHeight: 40, count: 0});

      expect(account.transport.subscribeToTransactionsWithProofs.secondCall.args[0].length).to.be.equal(40);
      expect(account.transport.subscribeToTransactionsWithProofs.secondCall.args[1]).to.be.deep.equal({ fromBlockHeight: 40, count: 0});

      expect(worker.stream).to.be.null;
    });
    it('should reconnect to the server closes the stream without any errors', async function () {
      const lastSavedBlockHeight = 40;
      const bestBlockHeight = 42;

      worker.setLastSyncedBlockHeight(lastSavedBlockHeight);

      account.transport.getBestBlockHeight
          .returns(bestBlockHeight);

      worker.execute();

      await wait(10);

      txStreamMock.emit(TxStreamMock.EVENTS.end);

      await wait(10);

      await worker.onStop();

      const addressesInStorage = storage.store.wallets[walletId].addresses.external;

      expect(Object.keys(addressesInStorage).length).to.be.equal(20);
      // It should reconnect if the server closes the stream
      expect(account.transport.subscribeToTransactionsWithProofs.callCount).to.be.equal(2);
      // 20 external and 20 internal
      expect(account.transport.subscribeToTransactionsWithProofs.firstCall.args[0].length).to.be.equal(40);
      expect(account.transport.subscribeToTransactionsWithProofs.firstCall.args[1]).to.be.deep.equal({ fromBlockHeight: 40, count: 0});

      expect(account.transport.subscribeToTransactionsWithProofs.secondCall.args[0].length).to.be.equal(40);
      expect(account.transport.subscribeToTransactionsWithProofs.secondCall.args[1]).to.be.deep.equal({ fromBlockHeight: 40, count: 0});

      expect(worker.stream).to.be.null;
    });

    // TODO: enable once we figure out what is the source of the problem
    it.skip('should not reconnect to the incoming stream if stream in case of any other error', async function () {
      const lastSavedBlockHeight = 40;
      const bestBlockHeight = 42;

      worker.setLastSyncedBlockHeight(lastSavedBlockHeight);

      account.transport.getBestBlockHeight
          .returns(bestBlockHeight);

      worker.execute();

      await wait(10);

      txStreamMock.emit(TxStreamMock.EVENTS.error, new Error('Some random error'));

      await worker.onStop();

      await expect(worker.incomingSyncPromise).to.be.rejectedWith('Some random error');

      const addressesInStorage = storage.store.wallets[walletId].addresses.external;
      expect(Object.keys(addressesInStorage).length).to.be.equal(20);

      // Shouldn't try to reconnect
      expect(account.transport.subscribeToTransactionsWithProofs.callCount).to.be.equal(1);
      // 20 external and 20 internal
      expect(account.transport.subscribeToTransactionsWithProofs.firstCall.args[0].length).to.be.equal(40);
      expect(account.transport.subscribeToTransactionsWithProofs.firstCall.args[1]).to.be.deep.equal({ fromBlockHeight: 40, count: 0});

      expect(worker.stream).to.be.null;
    });
  });

  it('should propagate instant locks', async () => {
    const transactions = [
      new Transaction().to(addressAtIndex19, 10000),
      new Transaction().to(account.getAddress(10).address, 10000),
      new Transaction().to(account.getAddress(11).address, 10000)
    ];

    const receivedInstantLocks = [];

    transactions.forEach(tx => {
      account.subscribeToTransactionInstantLock(tx.hash, (isLock) => {
        receivedInstantLocks.push(isLock);
      });
    });

    const instantLock1 = InstantLock.fromObject({
      inputs: [
        {
          outpointHash: '6e200d059fb567ba19e92f5c2dcd3dde522fd4e0a50af223752db16158dabb1d',
          outpointIndex: 0,
        },
      ],
      txid: transactions[0].hash,
      signature: '8967c46529a967b3822e1ba8a173066296d02593f0f59b3a78a30a7eef9c8a120847729e62e4a32954339286b79fe7590221331cd28d576887a263f45b595d499272f656c3f5176987c976239cac16f972d796ad82931d532102a4f95eec7d80',
    });
    const instantLock2 = InstantLock.fromObject({
      inputs: [
        {
          outpointHash: '6e200d059fb567ba19e92f5c2dcd3dde522fd4e0a50af223752db16158dabb1d',
          outpointIndex: 0,
        },
      ],
      txid: transactions[1].hash,
      signature: '8967c46529a967b3822e1ba8a173066296d02593f0f59b3a78a30a7eef9c8a120847729e62e4a32954339286b79fe7590221331cd28d576887a263f45b595d499272f656c3f5176987c976239cac16f972d796ad82931d532102a4f95eec7d80',
    });
    const lastSavedBlockHeight = 40;
    const bestBlockHeight = 42;

    worker.setLastSyncedBlockHeight(lastSavedBlockHeight);
    const transactionsSent = [];

    account.transport.getBestBlockHeight
        .returns(bestBlockHeight);

    worker.execute();

    await wait(10);

    try {
      let transaction = transactions[0];
      account.transport.getTransaction
          .returns({
            transaction:transactions[0],
            blockHash: Buffer.from('4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176', 'hex'),
            height: 42,
            confirmations: 10,
            isInstantLocked: true,
            isChainLocked: false,
          });

      transactionsSent.push(transaction);
      txStreamMock.emit(TxStreamMock.EVENTS.data, new TxStreamDataResponseMock({
        rawTransactions: [transaction.toBuffer()]
      }));

      txStreamMock.emit(
        TxStreamMock.EVENTS.data,
        new TxStreamDataResponseMock(
            { instantSendLockMessages: [instantLock1.toBuffer()] }
        )
      );

      await wait(10);

      merkleBlockMock.hashes[0] =  Buffer.from(transaction.hash, 'hex').reverse().toString('hex');
      txStreamMock.emit(TxStreamMock.EVENTS.data, new TxStreamDataResponseMock({
        rawMerkleBlock: merkleBlockMock.toBuffer()
      }));

      await wait(10);

      transaction = transactions[1];
      account.transport.getTransaction
          .returns({
            transaction:transactions[1],
            blockHash: Buffer.from('4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176', 'hex'),
            height: 42,
            confirmations: 10,
            isInstantLocked: true,
            isChainLocked: false,
          });

      account.transport.getBlockHeaderByHash
      .returns(new Buffer.from(blockHeaderFixture, 'hex'));


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
    expect(account.transport.subscribeToTransactionsWithProofs.callCount).to.be.equal(3);
    // 20 external and 20 internal
    expect(account.transport.subscribeToTransactionsWithProofs.firstCall.args[1]).to.be.deep.equal({ fromBlockHeight: 40, count: 0});
    expect(account.transport.subscribeToTransactionsWithProofs.firstCall.args[0].length).to.be.equal(40);
    // 20 more of each type, since the last address is used, Merkle Block received
    expect(account.transport.subscribeToTransactionsWithProofs.secondCall.args[0].length).to.be.equal(80);
    expect(account.transport.subscribeToTransactionsWithProofs.secondCall.args[1]).to.be.deep.equal({ fromBlockHash: '0000025d24ebe65454bd51a61bab94095a6ad1df996be387e31495f764d8e2d9', count: 0});

    expect(worker.stream).to.be.null;
    expect(transactionsInStorage.length).to.be.equal(2);
    expect(transactionsInStorage).to.have.deep.members(expectedTransactions);

    const [ actualLock ] = await Promise.all([
      account.waitForInstantLock(transactions[1].hash, 10000),
      new Promise((resolve => {
        setImmediate(() => {
          txStreamMock.emit(
              TxStreamMock.EVENTS.data,
              new TxStreamDataResponseMock(
                  { instantSendLockMessages: [instantLock2.toBuffer()] }
              )
          );
          resolve();
        })
      })),
    ]);

    expect(actualLock).to.be.deep.equal(instantLock2);
    expect(receivedInstantLocks.length).to.be.equal(2);
    expect(receivedInstantLocks[0]).to.be.deep.equal(instantLock1);
    expect(receivedInstantLocks[1]).to.be.deep.equal(instantLock2);

    // Test that if instant lock was already imported previously wait method will return it
    const firstISFromWait = await account.waitForInstantLock(transactions[0].hash);
    expect(firstISFromWait).to.be.deep.equal(instantLock1);

    // Check that wait method throws if timeout has passed
    await expect(account.waitForInstantLock(transactions[2].hash, 1000)).to.eventually
        .be.rejectedWith('InstantLock waiting period for transaction 256d5b3bf6d8869f5cc882ae070af9b648fa0f512bfa2b6f07b35d55e160a16c timed out');
  });

  it('should start from the height specified in `skipSynchronizationBeforeHeight` options', async function () {
    const bestBlockHeight = 42;

    wallet = new Wallet({
      HDPrivateKey: new HDPrivateKey(testHDKey),
      unsafeOptions: {
        skipSynchronizationBeforeHeight: 20,
      },
    });

    await createAndAttachTransportMocksToWallet(wallet, this.sinonSandbox);

    account = await wallet.getAccount();

    account.transport.getBestBlockHeight.resolves(bestBlockHeight);
    account.transport.getTransaction.returns({
      transaction:new Transaction().to(account.getAddress(10).address, 10000),
      blockHash: Buffer.from('4f46066bd50cc2684484407696b7949e82bd906ea92c040f59a97cba47ed8176', 'hex'),
      height: 42,
      confirmations: 10,
      isInstantLocked: true,
      isChainLocked: false,
    })

    expect(account.transport.subscribeToTransactionsWithProofs.getCall(0).args[1]).to.be.deep.equal({ fromBlockHeight: 20, count: bestBlockHeight - 20 });
  });
});
