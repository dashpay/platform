const { expect } = require('chai');
const updateAddress = require('../../../src/types/Storage/methods/updateAddress');
const orangeWStore = require('../../fixtures/walletStore').valid.orange.store;

describe('Storage - updateAddress', () => {
  it('should throw errors on failed update', () => {
    const self = {};
    expect(() => updateAddress.call(self)).to.throw('Expected walletId to update an address');

    expect(() => updateAddress.call(self, {}, '123ae')).to.throw('Address should have property path of type string');
  });
  it('should update an address', () => {
    const self = { store: orangeWStore, mappedAddress: {} };
    const validAddrObj = {
      path: "m/44'/1'/0'/0/0",
      index: '0',
      address: 'yLhsYLXW5sFHLDPLj2EHgrmQRhP712ANda',
      transactions: [],
      balanceSat: 0,
      unconfirmedBalanceSat: 0,
      utxos: {},
      fetchedLast: 0,
      used: true,
    };
    const validWalletId = Object.keys(orangeWStore.wallets)[0];
    updateAddress.call(self, validAddrObj, validWalletId);
    const expectedMappedAddress = { yLhsYLXW5sFHLDPLj2EHgrmQRhP712ANda: { walletId: 'a3771aaf93', type: 'external', path: "m/44'/1'/0'/0/0" } };
    const expectedUpdatedAddress = {
      path: "m/44'/1'/0'/0/0", index: '0', address: 'yLhsYLXW5sFHLDPLj2EHgrmQRhP712ANda', transactions: [], balanceSat: 0, unconfirmedBalanceSat: 0, utxos: {}, fetchedLast: 0, used: true,
    };
    expect(self.mappedAddress).to.be.deep.equal(expectedMappedAddress);
    expect(self.store.wallets.a3771aaf93.addresses.external["m/44'/1'/0'/0/0"]).to.deep.equal(expectedUpdatedAddress);
  });
});
