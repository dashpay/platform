const { is } = require('../../../utils');
const DerivableKeyChain = require('../../DerivableKeyChain/DerivableKeyChain');
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

  const keyChain = new DerivableKeyChain({ publicKey, network });
  this.keyChainStore = new KeyChainStore();
  this.keyChainStore.addKeyChain(keyChain, { isMasterKeyChain: true });
};
