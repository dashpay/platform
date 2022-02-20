const {Transaction, BlockHeader} = require('@dashevo/dashcore-lib');
const ChainStore = require('./ChainStore');
const {expect} = require('chai');
const fixtures1 = require('../../../fixtures/wallets/2a331817b9d6bf85100ef0/chain-store.json')
const fixtures2 = require('../../../fixtures/wallets/apart-trip-dignity/chain-store.json')

describe('ChainStore - class', function suite() {
  let testnetChainStore;
  let mainnetChainStore;
  it('should create a new chain store', function () {
    testnetChainStore = new ChainStore('testnet');
    mainnetChainStore = new ChainStore('mainnet');
    expect(new ChainStore()).to.deep.equal(testnetChainStore);
    expect(testnetChainStore.state).to.exist;
    expect(testnetChainStore.state.blockHeight).to.equal(0);
    expect(testnetChainStore.state.fees).to.deep.equal({minRelay: -1});
    expect(testnetChainStore.state.blockHeaders).to.deep.equal(new Map());
    expect(testnetChainStore.state.transactions).to.deep.equal(new Map());
    expect(testnetChainStore.state.instantLocks).to.deep.equal(new Map());
    expect(testnetChainStore.state.addresses).to.deep.equal(new Map());
  });
  it('should be able to import transactions with metadata', function () {
    let {transactions} = fixtures1.state;

    const tx1 = new Transaction(transactions.d48f415f08fb795d43b216cf56e9ef10e059d4009cfc8fc90edfc0d3850813af.transaction);
    const meta1 = transactions.d48f415f08fb795d43b216cf56e9ef10e059d4009cfc8fc90edfc0d3850813af.metadata;
    testnetChainStore.importTransaction(tx1, meta1);

    const storedTransactionData = testnetChainStore.getTransaction('d48f415f08fb795d43b216cf56e9ef10e059d4009cfc8fc90edfc0d3850813af');
    expect(storedTransactionData.transaction.toString()).to.equal(tx1.toString())
    expect(storedTransactionData.metadata).to.deep.equal(meta1)
  });
  it('should be able to import transaction without metadata', function () {
    let {transactions} = fixtures1.state;

    const tx1 = new Transaction(transactions['0dcdaa9bf5b3596be1bcf22113e39026fd49d24b47190e2c7423be936cb116a7'].transaction);
    testnetChainStore.importTransaction(tx1);

    const storedTransactionData = testnetChainStore.getTransaction('0dcdaa9bf5b3596be1bcf22113e39026fd49d24b47190e2c7423be936cb116a7');
    expect(storedTransactionData.transaction.toString()).to.equal(tx1.toString())
    expect(storedTransactionData.metadata).to.deep.equal({
      blockHash: null,
      height: null,
      isInstantLocked: null,
      isChainLocked: null
    })
  });
  it('should update metadata', function () {
    let {transactions} = fixtures1.state;

    const tx1 = new Transaction(transactions['0dcdaa9bf5b3596be1bcf22113e39026fd49d24b47190e2c7423be936cb116a7'].transaction);
    const meta1 = transactions['0dcdaa9bf5b3596be1bcf22113e39026fd49d24b47190e2c7423be936cb116a7'].metadata;
    testnetChainStore.importTransaction(tx1, meta1);
    const storedTransactionData = testnetChainStore.getTransaction('0dcdaa9bf5b3596be1bcf22113e39026fd49d24b47190e2c7423be936cb116a7');
    expect(storedTransactionData.metadata).to.deep.equal(meta1)
  });
  it('should be able to import and get a blockheader', function () {
    let {blockHeaders} = fixtures1.state;

    const blockheaders1 = new BlockHeader.fromString(blockHeaders['0000012464fba1e3c66e678de79e4003bf17c36d5caa689e80fd4711fe620ec1']);
    testnetChainStore.importBlockHeader(blockheaders1);

    const storedTransactionData = testnetChainStore.getBlockHeader('0000012464fba1e3c66e678de79e4003bf17c36d5caa689e80fd4711fe620ec1');
    expect(storedTransactionData.toString()).to.equal(blockheaders1.toString())
  });
  it('should be able to import addresses', function () {
    const { addresses, transactions } = fixtures1.state;

    testnetChainStore.importAddress('ycDeuTfs4U77bTb5cq17dame28zdWHVYfk')

    const tx1 = new Transaction(transactions['47d13f7f713f4258953292c2298c1d91e2d6dee309d689f3c8b44ccf457bab52'].transaction);
    const meta1 = transactions['47d13f7f713f4258953292c2298c1d91e2d6dee309d689f3c8b44ccf457bab52'].metadata;
    testnetChainStore.importTransaction(tx1, meta1);

    const addr = testnetChainStore.getAddress('ycDeuTfs4U77bTb5cq17dame28zdWHVYfk');
    expect(addr).to.deep.equal(addresses['ycDeuTfs4U77bTb5cq17dame28zdWHVYfk'])

  });
  it('should export and import state', function () {
    const exportedState = testnetChainStore.exportState();
    const importedChainStore = new ChainStore();
    importedChainStore.importState(exportedState);

    expect(importedChainStore.network).to.equal(testnetChainStore.network)
    expect(importedChainStore.state.fees).to.deep.equal(testnetChainStore.state.fees)
    expect(importedChainStore.state.blockHeight).to.deep.equal(testnetChainStore.state.blockHeight)
    expect(importedChainStore.state.blockHeaders).to.deep.equal(testnetChainStore.state.blockHeaders)
    expect(importedChainStore.state.instantLocks).to.deep.equal(testnetChainStore.state.instantLocks)
    expect([...importedChainStore.state.addresses]).to.deep.equal([...testnetChainStore.state.addresses])

    const importedTransactionsState = [...importedChainStore.state.transactions];
    const exportedTransactionsState = exportedState.state.transactions;

    importedTransactionsState.forEach(([hash, {transaction, metadata}]) => {
      expect(exportedTransactionsState[hash]).to.deep.equal({
        transaction: transaction.toString(),
        metadata
      })
    })
  });
});
