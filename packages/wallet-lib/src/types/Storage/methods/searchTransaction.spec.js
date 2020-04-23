const { expect } = require('chai');
const searchTransaction = require('./searchTransaction');
const transactionsFixtures = require('../../../../fixtures/transactions');

const { faa430b0fe84a074d981e6fa3995a13363478415ca029a12f6432bf3d90dfa60, fd7c727155ef67fd5c1d54b73dea869e9690c439570063d6e96fec1d3bba450e } = transactionsFixtures.valid.mainnet;
describe('Storage - searchTransaction', () => {
  it('should find a transaction', () => {
    const self = {
      store: {
        transactions: {
          faa430b0fe84a074d981e6fa3995a13363478415ca029a12f6432bf3d90dfa60,
        },
      },
    };

    self.getStore = () => self.store;

    const existingTxID = faa430b0fe84a074d981e6fa3995a13363478415ca029a12f6432bf3d90dfa60.hash;
    const notExistingTxID = fd7c727155ef67fd5c1d54b73dea869e9690c439570063d6e96fec1d3bba450e.hash;
    const search = searchTransaction.call(self, existingTxID);

    expect(search.found).to.be.equal(true);
    expect(search.hash).to.be.equal(existingTxID);
    expect(search.result).to.be.equal(faa430b0fe84a074d981e6fa3995a13363478415ca029a12f6432bf3d90dfa60);

    const search2 = searchTransaction.call(self, notExistingTxID);
    expect(search2.found).to.be.equal(false);
    expect(search2.hash).to.be.equal(notExistingTxID);
  });
});
