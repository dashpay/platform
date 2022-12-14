const { Transaction } = require('@dashevo/dashcore-lib');
const { expect } = require('chai');
const ChainStore = require('./ChainStore');
const fixtures1 = require('../../../fixtures/wallets/2a331817b9d6bf85100ef0/chain-store.json');
const { mockHeadersChain } = require("../../test/mocks/dashcore/block");

describe('ChainStore - class', () => {
  let testnetChainStore;

  it('should create a new chain store', () => {
    testnetChainStore = new ChainStore('testnet');

    expect(new ChainStore()).to.deep.equal(testnetChainStore);
    expect(testnetChainStore.state).to.exist;
    expect(testnetChainStore.state.chainHeight).to.equal(0);
    expect(testnetChainStore.state.lastSyncedBlockHeight).to.equal(-1);
    expect(testnetChainStore.state.lastSyncedHeaderHeight).to.equal(-1);
    expect(testnetChainStore.state.fees).to.deep.equal({ minRelay: -1 });
    expect(testnetChainStore.state.blockHeaders).to.deep.equal([]);
    expect(testnetChainStore.state.transactions).to.deep.equal(new Map());
    expect(testnetChainStore.state.instantLocks).to.deep.equal(new Map());
    expect(testnetChainStore.state.addresses).to.deep.equal(new Map());
  });
  it('should be able to import transactions with metadata', () => {
    const { transactions, txMetadata, lastSyncedBlockHeight, lastSyncedHeaderHeight, chainHeight } = fixtures1;

    const tx1 = new Transaction(
      transactions.d48f415f08fb795d43b216cf56e9ef10e059d4009cfc8fc90edfc0d3850813af,
    );
    const meta1 = txMetadata.d48f415f08fb795d43b216cf56e9ef10e059d4009cfc8fc90edfc0d3850813af;
    testnetChainStore.updateLastSyncedBlockHeight(lastSyncedBlockHeight);
    testnetChainStore.updateLastSyncedHeaderHeight(lastSyncedHeaderHeight);
    testnetChainStore.updateChainHeight(lastSyncedHeaderHeight);
    testnetChainStore.importTransaction(tx1, meta1);
    testnetChainStore.considerTransaction(tx1.hash);

    const storedTransactionData = testnetChainStore.getTransaction('d48f415f08fb795d43b216cf56e9ef10e059d4009cfc8fc90edfc0d3850813af');
    expect(storedTransactionData.transaction.toString()).to.equal(tx1.toString());
    expect(storedTransactionData.metadata).to.deep.equal({
      ...meta1,
      time: new Date(meta1.time)
    });
  });
  it('should be able to import transaction without metadata', () => {
    const { transactions } = fixtures1;

    const tx1 = new Transaction(transactions['0dcdaa9bf5b3596be1bcf22113e39026fd49d24b47190e2c7423be936cb116a7']);
    testnetChainStore.importTransaction(tx1);
    testnetChainStore.considerTransaction(tx1.hash);

    const storedTransactionData = testnetChainStore.getTransaction('0dcdaa9bf5b3596be1bcf22113e39026fd49d24b47190e2c7423be936cb116a7');
    expect(storedTransactionData.transaction.toString()).to.equal(tx1.toString());
    expect(storedTransactionData.metadata).to.deep.equal({
      blockHash: null,
      height: null,
      isInstantLocked: false,
      isChainLocked: false,
      time: null
    });
  });
  it('should update metadata', () => {
    const { transactions, txMetadata } = fixtures1;

    const tx1 = new Transaction(transactions['0dcdaa9bf5b3596be1bcf22113e39026fd49d24b47190e2c7423be936cb116a7']);
    const meta1 = txMetadata['0dcdaa9bf5b3596be1bcf22113e39026fd49d24b47190e2c7423be936cb116a7'];
    testnetChainStore.importTransaction(tx1, meta1);
    testnetChainStore.considerTransaction(tx1.hash);
    const storedTransactionData = testnetChainStore.getTransaction('0dcdaa9bf5b3596be1bcf22113e39026fd49d24b47190e2c7423be936cb116a7');
    expect(storedTransactionData.metadata).to.deep.equal({
      ...meta1,
      time: new Date(meta1.time)
    });
  });

  it('should export and import state', () => {
    const exportedState = testnetChainStore.exportState();
    const importedChainStore = new ChainStore();
    importedChainStore.importState(exportedState);

    expect(importedChainStore.state.blockHeaders)
      .to.deep.equal(testnetChainStore.state.blockHeaders);
    expect(importedChainStore.state.instantLocks)
      .to.deep.equal(testnetChainStore.state.instantLocks);

    const expectedTransactions = testnetChainStore.state.transactions;
    const importedTransactions = importedChainStore.state.transactions;

    expect(importedTransactions.size).to.equal(expectedTransactions.size);

    Array.from(expectedTransactions.keys()).forEach((txHash) => {
      expect(importedTransactions.has(txHash)).to.equal(true);
      expect(importedTransactions.get(txHash).transaction.toString())
        .to.equal(expectedTransactions.get(txHash).transaction.toString());

      expect(importedTransactions.get(txHash).metadata)
        .to.deep.equal(expectedTransactions.get(txHash).metadata);
    });
  });

  context('Handling headers metadata', () => {
    let headers;
    let chainStore;
    const TAIL_HEIGHT = 100;

    before(async () => {
      headers = mockHeadersChain('livenet', 10);

    })

    beforeEach(() => {
      chainStore = new ChainStore('livenet');
    })

    it('should update headers metadata', async () => {
      chainStore.updateHeadersMetadata(headers, TAIL_HEIGHT);

      const expectedMetadata = new Map();
      const expectedHashesByHeight = new Map();

      headers.forEach((header, index) => {
        const height = TAIL_HEIGHT - headers.length + index + 1;

        expectedHashesByHeight.set(height, header.hash);
        expectedMetadata.set(header.hash, {
          height,
          time: header.time,
        });
      }, new Map());

      expect(chainStore.state.headersMetadata).to.deep.equal(expectedMetadata)
      expect(chainStore.state.hashesByHeight).to.deep.equal(expectedHashesByHeight)
    });

    it('should prune headers metadata below specified height', () => {
      chainStore.updateHeadersMetadata(headers, TAIL_HEIGHT);

      let removeBelowHeight = TAIL_HEIGHT - 5;
      let remainingAmount = TAIL_HEIGHT - removeBelowHeight + 1;

      // Prune first 4 headers
      chainStore.pruneHeadersMetadata(removeBelowHeight)
      let expectedMetadata = new Map();
      let expectedHashesByHeight = new Map();

      let headHeight = removeBelowHeight;
      headers.slice(headers.length - remainingAmount).forEach((header, index) => {
        const height = headHeight + index;

        expectedHashesByHeight.set(height, header.hash);
        expectedMetadata.set(header.hash, {
          height,
          time: header.time,
        });
      }, new Map());

      let { headersMetadata, hashesByHeight } = chainStore.state;
      expect(headersMetadata.size).to.equal(remainingAmount);
      expect(hashesByHeight.size).to.equal(remainingAmount);
      expect(headersMetadata).to.deep.equal(expectedMetadata)
      expect(hashesByHeight).to.deep.equal(expectedHashesByHeight)

      expectedMetadata.clear();
      expectedHashesByHeight.clear();

      removeBelowHeight = TAIL_HEIGHT - 1;
      remainingAmount = TAIL_HEIGHT - removeBelowHeight + 1;

      // Prune last 5 headers
      chainStore.pruneHeadersMetadata(removeBelowHeight)

      headHeight = removeBelowHeight;
      headers.slice(headers.length - remainingAmount).forEach((header, index) => {
        const height = headHeight + index;

        expectedHashesByHeight.set(height, header.hash);
        expectedMetadata.set(header.hash, {
          height,
          time: header.time,
        });
      }, new Map());

      expect(headersMetadata.size).to.equal(remainingAmount);
      expect(hashesByHeight.size).to.equal(remainingAmount);
      expect(chainStore.state.headersMetadata).to.deep.equal(expectedMetadata)
      expect(chainStore.state.hashesByHeight).to.deep.equal(expectedHashesByHeight)
    });
  });
});
