const { is } = require('../../../utils');
const KeyChain = require('../../KeyChain/KeyChain');
const { WALLET_TYPES } = require('../../../CONSTANTS');
const KeyChainStore = require('../../KeyChainStore/KeyChainStore');

/**
 * @param address
 */
module.exports = function fromAddress(address, network) {
  if (!is.address(address)) throw new Error('Expected a valid address (typeof Address or String)');
  this.walletType = WALLET_TYPES.ADDRESS;
  this.mnemonic = null;
  this.address = address.toString();

  const keyChain = new KeyChain({ address, network });
  this.keyChainStore = new KeyChainStore();
  this.keyChainStore.addKeyChain(keyChain, { isMasterKeyChain: true });
};
