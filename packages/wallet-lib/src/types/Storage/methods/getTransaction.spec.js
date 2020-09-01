const { expect } = require('chai');
const getTransaction = require('./getTransaction');
const transactionsFixtures = require('../../../../fixtures/transactions');

describe('Storage - getTransaction', function suite() {
  this.timeout(10000);
  it('should throw on failed fetching', () => {
    const validTx = transactionsFixtures.valid.mainnet['4f71db0c4bf3e2769a3ebd2162753b54b33028e3287e45f93c5c7df8bac5ec7e'];
    const exceptedException1 = `Transaction is not in store: ${validTx.txid}`;
    const self = {
      store: {
        transactions: {},
      },
      searchTransaction: () => ({ found: false }),
    };
    expect(() => getTransaction.call(self, validTx.txid)).to.throw(exceptedException1);
  });
  it('should work', () => {
    const validTx = transactionsFixtures.valid.mainnet['4f71db0c4bf3e2769a3ebd2162753b54b33028e3287e45f93c5c7df8bac5ec7e'];
    const self = {
      store: {
        transactions: {},
      },
      searchTransaction: () => ({ found: true, result: validTx }),
    };
    self.store.transactions[validTx.txid] = validTx;
    const { txid } = validTx;
    expect(getTransaction.call(self, txid)).to.deep.equal(validTx);
  });
});
