const { expect } = require('chai');
const fromSeed = require('../../src/Wallet/fromSeed');
const knifeFixture = require('../fixtures/knifeeasily');
const { WALLET_TYPES } = require('../../src/CONSTANTS');

describe('Wallet - fromSeed', () => {
  it('should indicate missing data', () => {
    const mockOpts1 = { };
    const exceptedException1 = 'Expected a valid seed (typeof HDPrivateKey or String)';
    expect(() => fromSeed.call(mockOpts1)).to.throw(exceptedException1);
  });
  it('should set wallet from a HDPrivateKey', () => {
    const self1 = {};
    fromSeed.call(self1, knifeFixture.HDRootPrivateKeyMainnet);
    expect(self1.type).to.equal(WALLET_TYPES.HDWALLET);
    expect(self1.mnemonic).to.equal(null);
    expect(self1.HDPrivateKey).to.equal(knifeFixture.HDRootPrivateKeyMainnet);
    expect(self1.keyChain.type).to.equal('HDRootKey');
    expect(self1.keyChain.HDRootKey).to.equal(knifeFixture.HDRootPrivateKeyMainnet);
    expect(self1.keyChain.keys).to.deep.equal({});

    const self2 = {};
    fromSeed.call(self2, knifeFixture.HDRootPrivateKeyMainnet);
    expect(self2.type).to.equal(WALLET_TYPES.HDWALLET);
    expect(self2.mnemonic).to.equal(null);
    expect(self2.HDPrivateKey).to.equal(knifeFixture.HDRootPrivateKeyMainnet);
    expect(self2.keyChain.type).to.equal('HDRootKey');
    expect(self2.keyChain.HDRootKey).to.equal(knifeFixture.HDRootPrivateKeyMainnet);
    expect(self2.keyChain.keys).to.deep.equal({});
  });
  it('should reject invalid mnemonic', () => {
    const invalidInputs = [
      { seed: true },
      { seed: false },
      { seed: 0 },
    ];

    return invalidInputs.forEach((invalidInput) => {
      const self = {};
      expect(() => fromSeed.call(self, invalidInput)).to.throw('Expected a valid seed (typeof HDPrivateKey or String)');
    });
  });
});
