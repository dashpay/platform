const { is } = require('../../../utils');
const KeyChain = require('../../KeyChain/KeyChain');
const { WALLET_TYPES } = require('../../../CONSTANTS');

/**
 * @param address
 */
module.exports = function fromAddress(address) {
  if (!is.address(address)) throw new Error('Expected a valid address (typeof Address or String)');
  this.walletType = WALLET_TYPES.ADDRESS;
  this.mnemonic = null;
  this.address = address.toString();
  this.keyChain = new KeyChain({ address });
};
