const { expect } = require('chai');
const getTransactionMetadata = require('./getTransactionMetadata');
const transactionsFixtures = require('../../../../fixtures/transactions');

describe('Storage - getTransactionMetadata', function suite() {
  this.timeout(10000);
  const validTxId = '1a74dc225b3336c4edb1f94c9ec2ed88fd0ef136866fda26f8a734924407b4d6';
  // const validTx = transactionsFixtures.valid.testnet[validTxId];
  const validMetadata = transactionsFixtures.valid.testnet.metadata[validTxId];

  it('should throw on failed fetching', () => {
    const exceptedException1 = `Transaction metadata is not in store: ${validTxId}`;
    const self = {
      store: {
        transactionsMetadata: {},
      },
      searchTransactionMetadata: () => ({ found: false }),
    };
    expect(() => getTransactionMetadata.call(self, validTxId)).to.throw(exceptedException1);
  });
  it('should work', () => {
    const validTx = transactionsFixtures.valid.mainnet['4f71db0c4bf3e2769a3ebd2162753b54b33028e3287e45f93c5c7df8bac5ec7e'];
    const self = {
      store: {
        transactionsMetadata: {},
      },
      searchTransactionMetadata: () => ({ found: true, result: validMetadata }),
    };
    expect(getTransactionMetadata.call(self, validTxId)).to.deep.equal(validMetadata);
  });
});
