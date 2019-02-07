const { WALLET_TYPES } = require('../CONSTANTS');

/**
 * Get a specific addresss based on the index and type of address.
 * @param index - The index on the type
 * @param type - default: external - Type of the address (external, internal, misc)
 * @return <AddressInfo>
 */
function getAddress(index = 0, _type = 'external') {
  // eslint-disable-next-line no-nested-ternary
  // console.log(index, _type)
  const type = (this.type === WALLET_TYPES.SINGLE_ADDRESS)
    ? 'misc'
    : ((_type) || 'external');

  // eslint-disable-next-line no-nested-ternary
  const path = (type === 'misc')
    ? '0'
    : ((_type === 'external') ? `${this.BIP44PATH}/0/${index}` : `${this.BIP44PATH}/1/${index}`);

  const { wallets } = this.storage.getStore();
  const addressType = wallets[this.walletId].addresses[type];
  // console.log(type, path)
  // console.log(addressType, path)
  return (addressType[path]) ? addressType[path] : this.generateAddress(path);
}
module.exports = getAddress;
