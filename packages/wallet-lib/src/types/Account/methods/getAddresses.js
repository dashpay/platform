const { WALLET_TYPES } = require('../../../CONSTANTS');

/**
 * Get all the addresses from the store from a given type
 * @param {AddressType} [addressType="external"] - Type of the address (external, internal, misc)
 * @return {[AddressObj]} address - All address matching the type
 */
function getAddresses(addressType = 'external') {
  const addressTypeIndex = (addressType === 'external') ? 0 : 1;

  const { addresses } = this.storage
    .getWalletStore(this.walletId)
    .getPathState(this.accountPath);

  const chainStore = this.storage.getChainStore(this.network);

  const baseAddressPath = ([WALLET_TYPES.HDPUBLIC, WALLET_TYPES.HDWALLET].includes(this.walletType))
    ? `m/${addressTypeIndex}` : '0';

  const typedAddresses = {};

  Object
    .entries(addresses)
    .forEach(([path, address]) => {
      if (path.startsWith(baseAddressPath)) {
        const index = parseInt(path.split('/').slice(-1)[0], 10);
        typedAddresses[path] = {
          index,
          path,
          ...chainStore.getAddress(address),
        };
      }
    });

  return typedAddresses;
}
module.exports = getAddresses;
