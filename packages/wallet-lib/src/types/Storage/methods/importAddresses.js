/**
 * Import one or multiple addresses to the store
 * @param {[AddressObj]} addresses
 * @param {string} walletId
 * @return {boolean}
 */
const importAddresses = function importAddresses(addresses, walletId) {
  if (!walletId) throw new Error('Expected walletId to import addresses');
  if (!this.searchWallet(walletId).found) {
    this.createWallet(walletId);
  }
  const type = addresses.constructor.name;
  if (type === 'Object') {
    if (addresses.path) {
      const address = addresses;
      this.importAddress(address, walletId);
    } else {
      const addressPaths = Object.keys(addresses);
      addressPaths.forEach((path) => {
        const address = addresses[path];
        this.importAddress(address, walletId);
      });
    }
  } else if (type === 'Array') {
    throw new Error('Not implemented. Please create an issue on github if needed.');
  } else {
    throw new Error('Not implemented. Please create an issue on github if needed.');
  }
  return true;
};
module.exports = importAddresses;
