const {
  mnemonicToHDPrivateKey,
  is,
} = require('../../../utils');
const DerivableKeyChain = require('../../DerivableKeyChain/DerivableKeyChain');
const KeyChainStore = require('../../KeyChainStore/KeyChainStore');
const { WALLET_TYPES } = require('../../../CONSTANTS');

/**
 * Will set a wallet to work with a mnemonic (keychain, walletType & HDPrivateKey)
 * @param mnemonic
 */
module.exports = function fromMnemonic(mnemonic, network, passphrase = '') {
  if (!is.mnemonic(mnemonic)) {
    throw new Error('Expected a valid mnemonic (typeof String or Mnemonic)');
  }
  const trimmedMnemonic = mnemonic.toString().trim();
  this.walletType = WALLET_TYPES.HDWALLET;
  // As we do not require the mnemonic except in this.exportWallet
  // users of wallet-lib are free to clear this prop at anytime.
  this.mnemonic = trimmedMnemonic;
  this.HDPrivateKey = mnemonicToHDPrivateKey(trimmedMnemonic, network, passphrase);

  this.keyChainStore = new KeyChainStore();
  const keyChain = new DerivableKeyChain({ HDPrivateKey: this.HDPrivateKey });
  this.keyChainStore.addKeyChain(keyChain, { isMasterKeyChain: true });
};
