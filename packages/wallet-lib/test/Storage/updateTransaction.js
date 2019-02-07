const { expect } = require('chai');
const updateTransaction = require('../../src/Storage/updateTransaction');
const orangeWStore = require('../fixtures/walletStore').valid.orange.store;

describe('Storage - updateTransaction', () => {
  it('should throw on failed update', () => {
    const exceptedException1 = 'Expected a transaction to update';
    const self = {
      store: {
        transactions: {},
      },
    };

    expect(() => updateTransaction.call(self, null)).to.throw(exceptedException1);
  });
  it('should work', () => {
    const self = {
      store: {
        transactions: {
          '688dd18dea2b6f3c2d3892d13b41922fde7be01cd6040be9f3568dafbf9b1a23': {
            something: true,
          },
        },
      },
    };
    const txObj = {
      txid: '688dd18dea2b6f3c2d3892d13b41922fde7be01cd6040be9f3568dafbf9b1a23',
      something: false,
    };
    const expected = { '688dd18dea2b6f3c2d3892d13b41922fde7be01cd6040be9f3568dafbf9b1a23': { something: false, txid: '688dd18dea2b6f3c2d3892d13b41922fde7be01cd6040be9f3568dafbf9b1a23' } };


    const update = updateTransaction.call(self, txObj);
    expect(update).to.equal(true);
    expect(self.store.transactions).to.deep.equal(expected);
  });
});
