const { WALLET_TYPES } = require('../../../CONSTANTS');

const getTypePathFromWalletType = (walletType, addressType = 'external', index, BIP44PATH) => {
  let type;
  let path;

  const addressTypeIndex = (addressType === 'external') ? 0 : 1;
  switch (walletType) {
    case WALLET_TYPES.HDWALLET:
      type = addressType;
      path = `${BIP44PATH}/${addressTypeIndex}/${index}`;
      break;
    case WALLET_TYPES.HDPUBLIC:
      type = 'external';
      path = `${BIP44PATH}/${addressTypeIndex}/${index}`;
      break;
    case WALLET_TYPES.SINGLE_ADDRESS:
    default:
      type = 'misc';
      path = '0';
  }
  return { type, path };
};
/**
 * Get a specific addresss based on the index and type of address.
 * @param {number} index - The index on the type
 * @param {AddressType} [_type="external"] - Type of the address (external, internal, misc)
 * @return <AddressInfo>
 */
function getAddress(index = 0, _type = 'external') {
  const { type, path } = getTypePathFromWalletType(this.walletType, _type, index, this.BIP44PATH);

  const { wallets } = this.storage.getStore();
  const matchingTypeAddresses = wallets[this.walletId].addresses[type];
  return (matchingTypeAddresses[path]) ? matchingTypeAddresses[path] : this.generateAddress(path);
}
module.exports = getAddress;
