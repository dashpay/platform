const { expect } = require('chai');
const { PrivateKey } = require('@dashevo/dashcore-lib');
const fromPublicKey = require('./fromPublicKey');
const cR4t6eFixture = require('../../../../fixtures/cR4t6e_pk');
const { WALLET_TYPES } = require('../../../CONSTANTS');
const cR4t6ePublicKey = new PrivateKey(cR4t6eFixture.privateKey).toPublicKey();

describe('Wallet - fromPublicKey', function suite() {
  this.timeout(10000);
  it('should indicate missing data', () => {
    const mockOpts1 = { };
    const exceptedException1 = 'Expected a valid public key (typeof PublicKey or String)';
    expect(() => fromPublicKey.call(mockOpts1)).to.throw(exceptedException1);
  });
  it('should set wallet from public Key', () => {
    const self1 = {};
    fromPublicKey.call(self1, cR4t6ePublicKey);
    expect(self1.walletType).to.equal(WALLET_TYPES.PUBLICKEY);
    expect(self1.mnemonic).to.equal(null);
    expect(self1.publicKey).to.equal(cR4t6ePublicKey);
    const keyChain = self1.keyChainStore.getMasterKeyChain()
    expect(keyChain.rootKeyType).to.equal('publicKey');
    expect(keyChain.rootKey.toString()).to.equal(cR4t6ePublicKey.toString());

    const self2 = {};
    fromPublicKey.call(self2, cR4t6ePublicKey.toString());
    expect(self2.walletType).to.equal(WALLET_TYPES.PUBLICKEY);
    expect(self2.mnemonic).to.equal(null);
    expect(self2.publicKey).to.equal(cR4t6ePublicKey.toString());
    const keyChain2 = self2.keyChainStore.getMasterKeyChain()
    expect(keyChain2.rootKeyType).to.equal('publicKey');
    expect(keyChain2.rootKey.toString()).to.equal(cR4t6ePublicKey.toString());
  });
  it('should reject invalid mnemonic', () => {
    const invalidInputs = [
      { privateKey: 0 },
      { privateKey: true },
      { privateKey: false },
    ];

    return invalidInputs.forEach((invalidInput) => {
      const self = {};
      expect(() => fromPublicKey.call(self, invalidInput)).to.throw('Expected a valid public key (typeof PublicKey or String)');
    });
  });
});
