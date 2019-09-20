const { expect } = require('chai');
const fromPrivateKey = require('../../../src/types/Wallet/methods/fromPrivateKey');
const cR4t6eFixture = require('../../fixtures/cR4t6e_pk');
const { WALLET_TYPES } = require('../../../src/CONSTANTS');

describe('Wallet - fromPrivateKey', () => {
  it('should indicate missing data', () => {
    const mockOpts1 = { };
    const exceptedException1 = 'Expected a valid private key (typeof PrivateKey or String)';
    expect(() => fromPrivateKey.call(mockOpts1)).to.throw(exceptedException1);
  });
  it('should set wallet from private Key', () => {
    const self1 = {};
    fromPrivateKey.call(self1, cR4t6eFixture.privateKey);
    expect(self1.walletType).to.equal(WALLET_TYPES.SINGLE_ADDRESS);
    expect(self1.mnemonic).to.equal(null);
    expect(self1.privateKey).to.equal(cR4t6eFixture.privateKey);
    expect(self1.keyChain.type).to.equal('privateKey');
    expect(self1.keyChain.privateKey).to.equal(cR4t6eFixture.privateKey);
    expect(self1.keyChain.keys).to.deep.equal({});

    const self2 = {};
    fromPrivateKey.call(self2, cR4t6eFixture.privateKey);
    expect(self2.walletType).to.equal(WALLET_TYPES.SINGLE_ADDRESS);
    expect(self2.mnemonic).to.equal(null);
    expect(self2.privateKey).to.equal(cR4t6eFixture.privateKey);
    expect(self2.keyChain.type).to.equal('privateKey');
    expect(self2.keyChain.privateKey).to.equal(cR4t6eFixture.privateKey);
    expect(self2.keyChain.keys).to.deep.equal({});
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
