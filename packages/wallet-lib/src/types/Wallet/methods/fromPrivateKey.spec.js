const { expect } = require('chai');
const fromPrivateKey = require('./fromPrivateKey');
const cR4t6eFixture = require('../../../../fixtures/cR4t6e_pk');
const { WALLET_TYPES } = require('../../../CONSTANTS');

describe('Wallet - fromPrivateKey', function suite() {
  this.timeout(10000);
  it('should indicate missing data', () => {
    const mockOpts1 = { };
    const exceptedException1 = 'Expected a valid private key (typeof PrivateKey or String)';
    expect(() => fromPrivateKey.call(mockOpts1)).to.throw(exceptedException1);
  });
  it('should set wallet from private Key', () => {
    const self1 = {};
    fromPrivateKey.call(self1, cR4t6eFixture.privateKey);
    expect(self1.walletType).to.equal(WALLET_TYPES.PRIVATEKEY);
    expect(self1.mnemonic).to.equal(null);
    expect(self1.privateKey).to.equal(cR4t6eFixture.privateKey);
    const keyChain = self1.keyChainStore.getMasterKeyChain()
    expect(keyChain.rootKeyType).to.equal('privateKey');
    expect(keyChain.rootKey.toWIF()).to.equal(cR4t6eFixture.privateKey);

    const self2 = {};
    fromPrivateKey.call(self2, cR4t6eFixture.privateKey);
    expect(self2.walletType).to.equal(WALLET_TYPES.PRIVATEKEY);
    expect(self2.mnemonic).to.equal(null);
    expect(self2.privateKey).to.equal(cR4t6eFixture.privateKey);
    const keyChain2 = self2.keyChainStore.getMasterKeyChain()
    expect(keyChain2.rootKeyType).to.equal('privateKey');
    expect(keyChain2.rootKey.toWIF()).to.equal(cR4t6eFixture.privateKey);
  });
  it('should reject invalid mnemonic', () => {
    const invalidInputs = [
      { privateKey: 0 },
      { privateKey: true },
      { privateKey: false },
    ];

    return invalidInputs.forEach((invalidInput) => {
      const self = {};
      expect(() => fromPrivateKey.call(self, invalidInput)).to.throw('Expected a valid private key (typeof PrivateKey or String)');
    });
  });
});
