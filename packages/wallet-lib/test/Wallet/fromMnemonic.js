const { expect } = require('chai');
const fromMnemonic = require('../../src/Wallet/fromMnemonic');
const knifeFixture = require('../fixtures/knifeeasily');
const { WALLET_TYPES } = require('../../src/CONSTANTS');
const Dashcore = require('@dashevo/dashcore-lib');

describe('Wallet - fromMnemonic', () => {
  it('should indicate missing data', () => {
    const mockOpts1 = { };
    const exceptedException1 = 'Expected a valid mnemonic (typeof String or Mnemonic)';
    expect(() => fromMnemonic.call(mockOpts1)).to.throw(exceptedException1);
  });
  it('should set wallet from mnemonic', () => {
    const self1 = {};
    fromMnemonic.call(self1, knifeFixture.mnemonic);
    expect(self1.walletType).to.equal(WALLET_TYPES.HDWALLET);
    expect(self1.mnemonic).to.equal(knifeFixture.mnemonic);
    expect(self1.HDPrivateKey.toString()).to.equal(knifeFixture.HDRootPrivateKeyMainnet);
    expect(new Dashcore.HDPrivateKey(self1.HDPrivateKey)).to.equal(self1.HDPrivateKey);
    expect(self1.keyChain.type).to.equal('HDPrivateKey');
    expect(self1.keyChain.HDPrivateKey.toString()).to.equal(knifeFixture.HDRootPrivateKeyMainnet);
    expect(self1.keyChain.keys).to.deep.equal({});


    const self2 = { network: Dashcore.Networks.testnet };
    fromMnemonic.call(self2, knifeFixture.mnemonic);
    expect(self2.walletType).to.equal(WALLET_TYPES.HDWALLET);
    expect(self2.mnemonic).to.equal(knifeFixture.mnemonic);
    expect(self2.HDPrivateKey.toString()).to.equal(knifeFixture.HDRootPrivateKeyTestnet);
    expect(new Dashcore.HDPrivateKey(self1.HDPrivateKey)).to.equal(self1.HDPrivateKey);
    expect(self2.keyChain.type).to.equal('HDPrivateKey');
    expect(self2.keyChain.HDPrivateKey.toString()).to.equal(knifeFixture.HDRootPrivateKeyTestnet);
    expect(self2.keyChain.keys).to.deep.equal({});
  });
  it('should reject invalid mnemonic', () => {
    const invalidInputs = [
      { mnemonic: 'knife easily prosper input concert merge prepare autumn pen blood glance chair' },
      { mnemonic: false },
      { mnemonic: true },
      { mnemonic: 0 },
    ];

    return invalidInputs.forEach((invalidInput) => {
      const self = {};
      expect(() => fromMnemonic.call(self, invalidInput)).to.throw('Expected a valid mnemonic (typeof String or Mnemonic)');
    });
  });
});
