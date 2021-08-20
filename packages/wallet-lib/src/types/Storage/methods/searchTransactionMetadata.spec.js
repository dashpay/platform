const { expect } = require('chai');
const searchTransactionMetadata = require('./searchTransactionMetadata');
const transactionsFixtures = require('../../../../fixtures/transactions');

const validTxId = '1a74dc225b3336c4edb1f94c9ec2ed88fd0ef136866fda26f8a734924407b4d6';
const validTx = transactionsFixtures.valid.testnet[validTxId];
const validMetadata = transactionsFixtures.valid.testnet.metadata[validTxId];
describe('Storage - searchTransactionMetadata', function suite() {
  this.timeout(10000);
  it('should find a transaction metadata', () => {
    const self = {
      store: {
        transactions: {
          "1a74dc225b3336c4edb1f94c9ec2ed88fd0ef136866fda26f8a734924407b4d6": validTx,
        },
        transactionsMetadata: {
          "1a74dc225b3336c4edb1f94c9ec2ed88fd0ef136866fda26f8a734924407b4d6": {hash:"1a74dc225b3336c4edb1f94c9ec2ed88fd0ef136866fda26f8a734924407b4d6",...validMetadata}
        }
      },
    };

    self.getStore = () => self.store;

    const existingTxID = "1a74dc225b3336c4edb1f94c9ec2ed88fd0ef136866fda26f8a734924407b4d6";
    const notExistingTxID = "fd7c727155ef67fd5c1d54b73dea869e9690c439570063d6e96fec1d3bba450e";
    const search = searchTransactionMetadata.call(self, existingTxID);

    expect(search.found).to.be.equal(true);
    expect(search.hash).to.be.equal(existingTxID);
    const expectedResult = {hash: "1a74dc225b3336c4edb1f94c9ec2ed88fd0ef136866fda26f8a734924407b4d6", ...validMetadata};
    expect(search.result).to.be.deep.equal(expectedResult);

    const search2 = searchTransactionMetadata.call(self, notExistingTxID);
    expect(search2.found).to.be.equal(false);
    expect(search2.hash).to.be.equal(notExistingTxID);
  });
});
