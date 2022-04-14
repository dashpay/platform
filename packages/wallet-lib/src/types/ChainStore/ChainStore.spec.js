const { Transaction, BlockHeader } = require('@dashevo/dashcore-lib');
const { expect } = require('chai');
const ChainStore = require('./ChainStore');
const fixtures1 = require('../../../fixtures/wallets/2a331817b9d6bf85100ef0/chain-store.json');

describe('ChainStore - class', () => {
  let testnetChainStore;

  it('should create a new chain store', () => {
    testnetChainStore = new ChainStore('testnet');

    expect(new ChainStore()).to.deep.equal(testnetChainStore);
    expect(testnetChainStore.state).to.exist;
    expect(testnetChainStore.state.blockHeight).to.equal(0);
    expect(testnetChainStore.state.fees).to.deep.equal({ minRelay: -1 });
    expect(testnetChainStore.state.blockHeaders).to.deep.equal(new Map());
    expect(testnetChainStore.state.transactions).to.deep.equal(new Map());
    expect(testnetChainStore.state.instantLocks).to.deep.equal(new Map());
    expect(testnetChainStore.state.addresses).to.deep.equal(new Map());
  });
  it('should be able to import transactions with metadata', () => {
    const { transactions, txMetadata } = fixtures1;

    const tx1 = new Transaction(
      transactions.d48f415f08fb795d43b216cf56e9ef10e059d4009cfc8fc90edfc0d3850813af,
    );
    const meta1 = txMetadata.d48f415f08fb795d43b216cf56e9ef10e059d4009cfc8fc90edfc0d3850813af;
    testnetChainStore.importTransaction(tx1, meta1);
    testnetChainStore.considerTransaction(tx1.hash);

    const storedTransactionData = testnetChainStore.getTransaction('d48f415f08fb795d43b216cf56e9ef10e059d4009cfc8fc90edfc0d3850813af');
    expect(storedTransactionData.transaction.toString()).to.equal(tx1.toString());
    expect(storedTransactionData.metadata).to.deep.equal(meta1);
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
    });
  });
  it('should update metadata', () => {
    const { transactions, txMetadata } = fixtures1;

    const tx1 = new Transaction(transactions['0dcdaa9bf5b3596be1bcf22113e39026fd49d24b47190e2c7423be936cb116a7']);
    const meta1 = txMetadata['0dcdaa9bf5b3596be1bcf22113e39026fd49d24b47190e2c7423be936cb116a7'];
    testnetChainStore.importTransaction(tx1, meta1);
    testnetChainStore.considerTransaction(tx1.hash);
    const storedTransactionData = testnetChainStore.getTransaction('0dcdaa9bf5b3596be1bcf22113e39026fd49d24b47190e2c7423be936cb116a7');
    expect(storedTransactionData.metadata).to.deep.equal(meta1);
  });
  it('should be able to import and get a blockheader', () => {
    const { blockHeaders } = fixtures1;

    const blockheaders1 = BlockHeader.fromString(
      blockHeaders['0000012464fba1e3c66e678de79e4003bf17c36d5caa689e80fd4711fe620ec1'],
    );
    testnetChainStore.importBlockHeader(blockheaders1);

    const storedTransactionData = testnetChainStore.getBlockHeader('0000012464fba1e3c66e678de79e4003bf17c36d5caa689e80fd4711fe620ec1');
    expect(storedTransactionData.toString()).to.equal(blockheaders1.toString());
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
});
