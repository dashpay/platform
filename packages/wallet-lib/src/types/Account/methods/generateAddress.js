const EVENTS = require('../../../EVENTS');
const { WALLET_TYPES } = require('../../../CONSTANTS');
const { is } = require('../../../utils');
/**
 * Generate an address from a path and import it to the store
 * @param {string} path
 * @return {AddressObj} Address information
 * */
function generateAddress(path) {
  if (is.undefOrNull(path)) throw new Error('Expected path to generate an address');
  let index = 0;
  let privateKey;

  const { network } = this;

  switch (this.walletType) {
    case WALLET_TYPES.HDWALLET:
      // eslint-disable-next-line prefer-destructuring
      index = parseInt(path.toString().split('/')[5], 10);
      privateKey = this.keyChain.getKeyForPath(path);
      break;
    case WALLET_TYPES.HDPUBLIC:
      index = parseInt(path.toString().split('/')[5], 10);
      privateKey = this.keyChain.getKeyForChild(index);
      break;
    case WALLET_TYPES.SINGLE_ADDRESS:
    default:
      privateKey = this.keyChain.getKeyForPath(path.toString());
  }
  const address = privateKey.publicKey.toAddress(network).toString();

  const addressData = {
    path: path.toString(),
    index,
    address,
    // privateKey,
    transactions: [],
    balanceSat: 0,
    unconfirmedBalanceSat: 0,
    utxos: {},
    fetchedLast: 0,
    used: false,
  };

  this.storage.importAddresses(addressData, this.walletId);
  this.emit(EVENTS.GENERATED_ADDRESS, { type: EVENTS.GENERATED_ADDRESS, payload: addressData });
  return addressData;
}
module.exports = generateAddress;
