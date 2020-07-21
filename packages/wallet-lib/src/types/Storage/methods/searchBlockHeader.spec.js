const {expect} = require('chai');
const {BlockHeader} = require('@dashevo/dashcore-lib');
const searchBlockHeader = require('./searchBlockHeader');

const existingBlockHeader = new BlockHeader.fromObject({
  hash: '00000ac3a0c9df709260e41290d6902e5a4a073099f11fe8c1ce80aadc4bb331',
  version: 2,
  prevHash: '00000ce430de949c85a145b02e33ebbaed3772dc8f3d668f66edc6852c24d002',
  merkleRoot: '663360403b5fba9cd8744c3706f9660c7d3fee4e5a9ee98ce0ad5e5ad7824c1d',
  time: 1398712821,
  bits: 504365040,
  nonce: 312363
});
const notExistingBlockHeader = new BlockHeader.fromObject({
  hash: '00000b526b34e733532d706c1f4cef93eefe707b87c2c3cb2978e1a84b97c501',
  version: 2,
  prevHash: '00000ac3a0c9df709260e41290d6902e5a4a073099f11fe8c1ce80aadc4bb331',
  merkleRoot: 'c2ed22a3e6712b842359dfbb6f0a133ae122ffb601e4cf60e30b8c99f9438f4f',
  time: 1398712821,
  bits: 504365040,
  nonce: 8325
})
describe('Storage - searchBlockHeader', function suite() {
  this.timeout(10000);
  it('should find a transaction', () => {
    const self = {
      network: 'testnet',
      store: {
        chains:{
          'testnet':{
            blockHeaders: {
              '00000ac3a0c9df709260e41290d6902e5a4a073099f11fe8c1ce80aadc4bb331': existingBlockHeader,
            },
          }
        },
      },
    };

    self.getStore = () => self.store;

    const existingBlockHash = existingBlockHeader.hash;
    const notExistingBlockHash = notExistingBlockHeader.hash;
    const search = searchBlockHeader.call(self, existingBlockHash);

    expect(search.found).to.be.equal(true);
    expect(search.identifier).to.be.equal(existingBlockHash);
    expect(search.result).to.be.deep.equal(existingBlockHeader);

    const search2 = searchBlockHeader.call(self, notExistingBlockHash);
    expect(search2.found).to.be.equal(false);
    expect(search2.identifier).to.be.equal(notExistingBlockHash);
  });
});
