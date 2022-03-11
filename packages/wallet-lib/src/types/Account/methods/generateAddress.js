const EVENTS = require('../../../EVENTS');
const { WALLET_TYPES } = require('../../../CONSTANTS');
const { is } = require('../../../utils');

/**
 * Generate an address from a path and import it to the store
 * @param {string} path
 * @param {boolean} [isWatchedAddress=true] - if the address will be watched
 * @return {AddressInfo} Address information
 * */
function generateAddress(path, isWatchedAddress = true) {
  if (is.undefOrNull(path)) throw new Error('Expected path to generate an address');
  let index = 0;
  let address;
  let keyPathData;
  const { network } = this;

  switch (this.walletType) {
    case WALLET_TYPES.ADDRESS:
      address = this.keyChainStore.getMasterKeyChain().rootKey;
      if (isWatchedAddress) {
        this.keyChainStore.issuedPaths.set(0, {
          path: 0,
          address,
          isUsed: false,
          isWatched: true,
        });
      }
      break;
    case WALLET_TYPES.PUBLICKEY:
      // eslint-disable-next-line no-case-declarations
      const { rootKey } = this.keyChainStore.getMasterKeyChain();
      address = rootKey.toAddress(network).toString();
      if (isWatchedAddress) {
        this.keyChainStore.issuedPaths.set(0, {
          key: rootKey,
          path: 0,
          address,
          isUsed: false,
          isWatched: true,
        });
      }
      break;
    case WALLET_TYPES.HDPRIVATE:
    case WALLET_TYPES.HDWALLET:
      // eslint-disable-next-line prefer-destructuring
      index = parseInt(path.toString().split('/')[2], 10);
      keyPathData = this.keyChainStore
        .getMasterKeyChain()
        .getForPath(path, { isWatched: isWatchedAddress });
      address = keyPathData.address.toString();
      break;
    case WALLET_TYPES.HDPUBLIC:
      index = parseInt(path.toString().split('/')[5], 10);
      // eslint-disable-next-line no-case-declarations
      keyPathData = this.keyChainStore
        .getMasterKeyChain()
        .getForPath(path, { isWatched: isWatchedAddress });
      address = keyPathData.address.toString();
      break;
    // TODO: DEPRECATE USAGE OF SINGLE_ADDRESS in favor or PRIVATEKEY
    case WALLET_TYPES.PRIVATEKEY:
    case WALLET_TYPES.SINGLE_ADDRESS:
    default:
      keyPathData = this.keyChainStore
        .getMasterKeyChain()
        .getForPath(path, { isWatched: isWatchedAddress });
      address = keyPathData.address.toString();
      break;
  }

  const addressData = {
    path: path.toString(),
    index,
    address,
    transactions: [],
    utxos: {},
    balanceSat: 0,
    unconfirmedBalanceSat: 0,
  };

  const accountStore = this.storage
    .getWalletStore(this.walletId)
    .getPathState(this.accountPath);

  const chainStore = this.storage.getChainStore(this.network);

  accountStore.addresses[addressData.path] = addressData.address.toString();
  chainStore.importAddress(addressData.address.toString());
  this.emit(EVENTS.GENERATED_ADDRESS, { type: EVENTS.GENERATED_ADDRESS, payload: addressData });
  return addressData;
}

module.exports = generateAddress;
