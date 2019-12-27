const Dashcore = require('@dashevo/dashcore-lib');
const EVENTS = require('../../../EVENTS');
const { WALLET_TYPES } = require('../../../CONSTANTS');
const { is } = require('../../../utils');
/**
 * Generate an address from a path and import it to the store
 * @param path
 * @return {addressObj} Address information
 * */
function generateAddress(path) {
  if (is.undefOrNull(path)) throw new Error('Expected path to generate an address');
  let index = 0;
  let privateKey;

  const { network } = this;

  switch (this.walletType) {
    case WALLET_TYPES.HDWALLET:
      // eslint-disable-next-line prefer-destructuring
      index = parseInt(path.split('/')[5], 10);
      privateKey = this.keyChain.getKeyForPath(path);
      break;
    case WALLET_TYPES.HDPUBLIC:
      index = parseInt(path.split('/')[5], 10);
      privateKey = this.keyChain.getKeyForChild(index);
      break;
    case WALLET_TYPES.SINGLE_ADDRESS:
    default:
      privateKey = this.keyChain.getKeyForPath(path);
  }

  const address = new Dashcore.Address(privateKey.publicKey.toAddress(network), network).toString();

  const addressData = {
    path,
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
  this.events.emit(EVENTS.GENERATED_ADDRESS, path);
  // console.log('gen', address,path)
  return addressData;
}
module.exports = generateAddress;
