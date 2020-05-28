const { expect } = require('chai');
const createChain = require('./createChain');

describe('Storage - createChain', function suite() {
  this.timeout(10000);
  it('should create a chain', () => {
    const self = {
      store: { chains: {} },
    };
    const testnet = 'testnet';

    createChain.call(self, testnet);

    const expected = {
      store: {
        chains: {
          testnet: {
            name: 'testnet', blockHeight: -1, blockHeaders: {}, mappedBlockHeaderHeights: {},
          },
        },
      },
    };
    expect(self).to.be.deep.equal(expected);
  });
});
