const { expect } = require('chai');
const logger = require('../../../logger');
const importAddress = require('./importAddress');

describe('Storage - importAddress', () => {
  it('should throw on failed import', () => {
    const walletId = '123ae';
    const exceptedException1 = 'Expected walletId to import addresses';

    expect(() => importAddress.call({})).to.throw(exceptedException1);
    expect(() => importAddress.call({}, walletId)).to.throw(exceptedException1);
  });
  it('should import an address', () => {
    logger.warn('FIXME');
    // const self = {};
    // importAddress.call(self, {});
  });
});
