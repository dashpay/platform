const { is } = require('../../../utils');
const KeyChain = require('../../KeyChain/KeyChain');
const { WALLET_TYPES } = require('../../../CONSTANTS');

/**
 * Will set a wallet to work with a mnemonic (keychain, walletType & HDPrivateKey)
 * @param privateKey
 */
module.exports = function fromPublicKey(publicKey) {
  if (!is.publicKey(publicKey)) throw new Error('Expected a valid public key (typeof PublicKey or String)');
  this.walletType = WALLET_TYPES.PUBLICKEY;
  this.mnemonic = null;
  this.publicKey = publicKey;
  this.keyChain = new KeyChain({ publicKey });
};
