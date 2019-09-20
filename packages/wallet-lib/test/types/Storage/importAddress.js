const { expect } = require('chai');
const importAddress = require('../../../src/types/Storage/methods/importAddress');

describe('Storage - importAddress', () => {
  it('should throw on failed import', () => {
    const mockOpts1 = { };
    const walletId = '123ae';
    const exceptedException1 = 'Expected walletId to import addresses';

    expect(() => importAddress.call({})).to.throw(exceptedException1);
    expect(() => importAddress.call({}, walletId)).to.throw(exceptedException1);
  });
  it('should import an address', () => {
    console.log('FIXME');
    // const self = {};
    // importAddress.call(self, {});
    // console.log(self);
  });
});
