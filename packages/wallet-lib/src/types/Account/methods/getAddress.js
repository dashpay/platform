const { WALLET_TYPES } = require('../../../CONSTANTS');

/**
 * Get a specific addresss based on the index and type of address.
 * @param {number} index - The index on the type
 * @param {AddressType} [addressType="external"] - Type of the address (external, internal, misc)
 * @return <AddressInfo>
 */
function getAddress(addressIndex = 0, addressType = 'external') {
  const addressTypeIndex = (addressType === 'external') ? 0 : 1;

  const { addresses } = this.storage.getWalletStore(this.walletId).getPathState(this.accountPath);
  const addressPath = ([WALLET_TYPES.HDPUBLIC, WALLET_TYPES.HDWALLET].includes(this.walletType))
    ? `m/${addressTypeIndex}/${addressIndex}` : '0';

  const address = addresses[addressPath];
  if (!address) return this.generateAddress(addressPath);

  const chainStore = this.storage.getChainStore(this.network);
  return {
    index: addressIndex,
    path: addressPath,
    ...chainStore.getAddress(address),
  };
}
module.exports = getAddress;
