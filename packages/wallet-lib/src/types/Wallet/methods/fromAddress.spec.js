const { expect } = require('chai');
const { PrivateKey } = require('@dashevo/dashcore-lib');
const fromAddress = require('./fromAddress');
const cR4t6eFixture = require('../../../../fixtures/cR4t6e_pk');
const { WALLET_TYPES } = require('../../../CONSTANTS');
const cR4t6ePublicKey = new PrivateKey(cR4t6eFixture.privateKey).toPublicKey();

describe('Wallet - fromAddress', function suite() {
  this.timeout(10000);
  it('should indicate missing data', () => {
    const mockOpts1 = { };
    const exceptedException1 = 'Expected a valid address (typeof Address or String)';
    expect(() => fromAddress.call(mockOpts1)).to.throw(exceptedException1);
  });
  it('should set wallet from address', () => {
    const self1 = {};
    fromAddress.call(self1, cR4t6ePublicKey.toAddress());
    expect(self1.walletType).to.equal(WALLET_TYPES.ADDRESS);
    expect(self1.mnemonic).to.equal(null);
    expect(self1.address).to.equal(cR4t6ePublicKey.toAddress().toString());
    expect(self1.keyChain.type).to.equal('address');
    expect(self1.keyChain.address).to.equal(cR4t6ePublicKey.toAddress().toString());
    expect(self1.keyChain.keys).to.deep.equal({});

    const self2 = {};
    fromAddress.call(self2, cR4t6ePublicKey.toAddress().toString());
    expect(self2.walletType).to.equal(WALLET_TYPES.ADDRESS);
    expect(self2.mnemonic).to.equal(null);
    expect(self2.address).to.equal(cR4t6ePublicKey.toAddress().toString());
    expect(self2.keyChain.type).to.equal('address');
    expect(self2.keyChain.address).to.equal(cR4t6ePublicKey.toAddress().toString());
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
      expect(() => fromAddress.call(self, invalidInput)).to.throw('Expected a valid address (typeof Address or String)');
    });
  });
});
