const Dashcore = require('@dashevo/dashcore-lib');
const EVENTS = require('../EVENTS');
const { WALLET_TYPES } = require('../CONSTANTS');

/**
 * Generate an address from a path and import it to the store
 * @param path
 * @return {addressObj} Address information
 * */
function generateAddress(path) {
  if (!path) throw new Error('Expected path to generate an address');
  let index = 0;
  const { network } = this;
  if (this.type === WALLET_TYPES.HDWALLET) {
    // eslint-disable-next-line prefer-destructuring
    index = path.split('/')[5];
  }

  const privateKey = this.keyChain.getKeyForPath(path);

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
