const { expect } = require('chai');
const fromSeed = require('./fromSeed');
const fromHDPrivateKey = require('./fromHDPrivateKey');
const knifeFixture = require('../../../../fixtures/knifeeasily');
const { WALLET_TYPES } = require('../../../CONSTANTS');

describe('Wallet - fromSeed', function suite() {
  this.timeout(10000);
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
    const keyChain = self1.keyChainStore.getMasterKeyChain()
    expect(keyChain.rootKeyType).to.equal('HDPrivateKey');
    expect(keyChain.rootKey.toString()).to.equal(knifeFixture.HDRootPrivateKeyTestnet);

    const self2 = {
      fromHDPrivateKey,
      network: 'mainnet',

    };
    fromSeed.call(self2, knifeFixture.seed, self2.network);
    expect(self2.walletType).to.equal(WALLET_TYPES.HDWALLET);
    expect(self2.mnemonic).to.equal(null);
    expect(self2.HDPrivateKey.toString()).to.equal(knifeFixture.HDRootPrivateKeyMainnet);
    const keyChain2 = self2.keyChainStore.getMasterKeyChain()
    expect(keyChain2.rootKeyType).to.equal('HDPrivateKey');
    expect(keyChain2.rootKey.toString()).to.equal(knifeFixture.HDRootPrivateKeyMainnet);
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
