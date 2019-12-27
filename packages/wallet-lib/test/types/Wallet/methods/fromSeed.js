const { expect } = require('chai');
const fromSeed = require('../../../../src/types/Wallet/methods/fromSeed');
const fromHDPrivateKey = require('../../../../src/types/Wallet/methods/fromHDPrivateKey');
const knifeFixture = require('../../../fixtures/knifeeasily');
const { WALLET_TYPES } = require('../../../../src/CONSTANTS');

describe('Wallet - fromSeed', () => {
  it('should indicate missing data', () => {
    const mockOpts1 = { };
    const exceptedException1 = 'Expected a valid seed (typeof string)';
    expect(() => fromSeed.call(mockOpts1)).to.throw(exceptedException1);
  });
  it('should set wallet from a HDPrivateKey', () => {
    const self1 = {
      fromHDPrivateKey,
    };
    fromSeed.call(self1, knifeFixture.seed);
    expect(self1.walletType).to.equal(WALLET_TYPES.HDWALLET);
    expect(self1.mnemonic).to.equal(null);
    expect(self1.HDPrivateKey.toString()).to.equal(knifeFixture.HDRootPrivateKeyTestnet);
    expect(self1.keyChain.type).to.equal('HDPrivateKey');
    expect(self1.keyChain.HDPrivateKey.toString()).to.equal(knifeFixture.HDRootPrivateKeyTestnet);
    expect(self1.keyChain.keys).to.deep.equal({});

    const self2 = {
      fromHDPrivateKey,
      network: 'mainnet',

    };
    fromSeed.call(self2, knifeFixture.seed);
    expect(self2.walletType).to.equal(WALLET_TYPES.HDWALLET);
    expect(self2.mnemonic).to.equal(null);
    expect(self2.HDPrivateKey.toString()).to.equal(knifeFixture.HDRootPrivateKeyMainnet);
    expect(self2.keyChain.type).to.equal('HDPrivateKey');
    expect(self2.keyChain.HDPrivateKey.toString()).to.equal(knifeFixture.HDRootPrivateKeyMainnet);
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
      expect(() => fromSeed.call(self, invalidInput)).to.throw('Expected a valid seed (typeof string)');
    });
  });
});
