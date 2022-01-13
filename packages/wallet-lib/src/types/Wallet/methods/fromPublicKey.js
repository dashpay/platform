const { is } = require('../../../utils');
const KeyChain = require('../../KeyChain/KeyChain');
const { WALLET_TYPES } = require('../../../CONSTANTS');
const KeyChainStore = require('../../KeyChainStore/KeyChainStore');

/**
 * Will set a wallet to work with a mnemonic (keychain, walletType & HDPrivateKey)
 * @param privateKey
 */
module.exports = function fromPublicKey(publicKey, network) {
  if (!is.publicKey(publicKey)) throw new Error('Expected a valid public key (typeof PublicKey or String)');
  this.walletType = WALLET_TYPES.PUBLICKEY;
  this.mnemonic = null;
  this.publicKey = publicKey;

  const keyChain = new KeyChain({ publicKey, network });
  this.keyChainStore = new KeyChainStore();
  this.keyChainStore.addKeyChain(keyChain, { isMasterKeyChain: true });
};
