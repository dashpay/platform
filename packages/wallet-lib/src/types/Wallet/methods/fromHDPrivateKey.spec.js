const { expect } = require('chai');
const fromHDPrivateKey = require('./fromHDPrivateKey');
const knifeFixture = require('../../../../fixtures/knifeeasily');
const { WALLET_TYPES } = require('../../../CONSTANTS');

describe('Wallet - fromHDPrivateKey', function suite() {
  this.timeout(10000);
  it('should indicate missing data', () => {
    const mockOpts1 = { };
    const exceptedException1 = 'Expected a valid HDPrivateKey (typeof HDPrivateKey or String)';
    expect(() => fromHDPrivateKey.call(mockOpts1)).to.throw(exceptedException1);
  });
  it('should set wallet from a HDPrivateKey', () => {
    const self1 = {};
    fromHDPrivateKey.call(self1, knifeFixture.HDRootPrivateKeyMainnet);
    expect(self1.walletType).to.equal(WALLET_TYPES.HDWALLET);
    expect(self1.mnemonic).to.equal(null);
    expect(self1.HDPrivateKey.toString()).to.equal(knifeFixture.HDRootPrivateKeyMainnet);

    const keyChain = self1.keyChainStore.getMasterKeyChain()
    expect(keyChain.rootKeyType).to.equal('HDPrivateKey');
    expect(keyChain.rootKey.toString()).to.equal(knifeFixture.HDRootPrivateKeyMainnet);
  });
  it('should reject invalid mnemonic', () => {
    const invalidInputs = [
      { seed: true },
      { seed: false },
      { seed: 0 },
    ];

    return invalidInputs.forEach((invalidInput) => {
      const self = {};
      expect(() => fromHDPrivateKey.call(self, invalidInput)).to.throw('Expected a valid HDPrivateKey (typeof HDPrivateKey or String)');
    });
  });
});
