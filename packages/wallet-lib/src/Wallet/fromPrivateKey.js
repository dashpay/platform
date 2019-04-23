const { is } = require('../utils/index');
const KeyChain = require('../KeyChain');
const { WALLET_TYPES } = require('../CONSTANTS');


/**
 * Will set a wallet to work with a mnemonic (keychain, walletType & HDPrivateKey)
 * @param privateKey
 */
module.exports = function fromPrivateKey(privateKey) {
  if (!is.privateKey(privateKey)) throw new Error('Expected a valid private key (typeof PrivateKey or String)');
  this.walletType = WALLET_TYPES.SINGLE_ADDRESS;
  this.mnemonic = null;
  this.privateKey = privateKey;
  this.keyChain = new KeyChain({ privateKey });
};
