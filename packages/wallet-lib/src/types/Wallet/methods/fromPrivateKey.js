const { is } = require('../../../utils');
const DerivableKeyChain = require('../../DerivableKeyChain/DerivableKeyChain');
const { WALLET_TYPES } = require('../../../CONSTANTS');
const KeyChainStore = require('../../KeyChainStore/KeyChainStore');

/**
 * Will set a wallet to work with a mnemonic (keychain, walletType & HDPrivateKey)
 * @param privateKey
 */
module.exports = function fromPrivateKey(privateKey, network) {
  if (!is.privateKey(privateKey)) throw new Error('Expected a valid private key (typeof PrivateKey or String)');
  this.walletType = WALLET_TYPES.PRIVATEKEY;
  this.mnemonic = null;
  this.privateKey = privateKey;

  const keyChain = new DerivableKeyChain({ privateKey, network });
  this.keyChainStore = new KeyChainStore();
  this.keyChainStore.addKeyChain(keyChain, { isMasterKeyChain: true });
};
