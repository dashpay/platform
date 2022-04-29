const { expect } = require('chai');
const getUnusedAddress = require('./getUnusedAddress');
const getFixtureHDAccountWithStorage = require('../../../../fixtures/wallets/apart-trip-dignity/getFixtureAccountWithStorage');

const mockedHDSelf = {
  ...getFixtureHDAccountWithStorage(),
}

describe('Account - getUnusedAddress', function suite() {
  this.timeout(10000);

  it('should get the proper unused address', () => {
    const unusedAddressExternal = getUnusedAddress.call(mockedHDSelf);
    const unusedAddressInternal = getUnusedAddress.call(mockedHDSelf, 'internal');

    expect(unusedAddressExternal).to.be.deep.equal({
      address: 'ybuL6rM6dgrKzCg8s99f3jxGuv5oz5JcDA',
      index: 3,
      path: 'm/0/3'
    });

    expect(unusedAddressInternal).to.be.deep.equal({
      address: 'yYwKP1FQae5kbjXkmuirGx6Xzf8NzHpLqW',
      path: 'm/1/4',
      index: 4
    });
  });
});
