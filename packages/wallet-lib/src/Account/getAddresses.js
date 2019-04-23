const { WALLET_TYPES } = require('../CONSTANTS');

/**
 * Get all the addresses from the store from a given type
 * @param type - default: external - Type of the address (external, internal, misc)
 * @return {Object} address - All address matching the type
 */
function getAddresses(_type = 'external') {
  const walletType = (this.walletType === WALLET_TYPES.SINGLE_ADDRESS)
    ? 'misc'
    : ((_type) || 'external');
  const store = this.storage.getStore();
  return store.wallets[this.walletId].addresses[walletType];
}
module.exports = getAddresses;
