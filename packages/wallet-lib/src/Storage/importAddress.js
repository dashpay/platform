const { cloneDeep } = require('lodash');
const { InvalidAddressObject } = require('../errors');
const { is } = require('../utils');
/**
 * Import one address to the store
 * @param addressObj
 * @param walletId
 * @return {boolean}
 */
const importAddress = function (addressObj, walletId) {
  if (!walletId) throw new Error('Expected walletId to import addresses');
  if (!this.searchWallet(walletId).found) {
    this.createWallet(walletId);
  }
  const addressesStore = this.store.wallets[walletId].addresses;
  if (is.undef(walletId)) throw new Error('Expected walletId to import an address');
  if (!is.addressObj(addressObj)) throw new InvalidAddressObject(addressObj);
  const { path } = addressObj;
  const modifiedAddressObject = cloneDeep(addressObj);
  const index = parseInt(path.split('/')[5], 10);
  const typeInt = path.split('/')[4];
  let type;
  switch (typeInt) {
    case '0':
      type = 'external';
      break;
    case '1':
      type = 'internal';
      break;
    default:
      type = 'misc';
  }
  if (!walletId) throw new Error('Invalid walletId. Cannot import');
  if (!modifiedAddressObject.index) modifiedAddressObject.index = index;
  if (addressesStore[type][path]) {
    if (addressesStore[type][path].fetchedLast < modifiedAddressObject.fetchedLast) {
      this.updateAddress(modifiedAddressObject, walletId);
    }
  } else {
    this.updateAddress(modifiedAddressObject, walletId);
  }
  return true;
};
module.exports = importAddress;
