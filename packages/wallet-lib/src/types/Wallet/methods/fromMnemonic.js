const {
  mnemonicToHDPrivateKey,
  is,
} = require('../../../utils');
const KeyChain = require('../../KeyChain/KeyChain');
const { WALLET_TYPES } = require('../../../CONSTANTS');

/**
 * Will set a wallet to work with a mnemonic (keychain, walletType & HDPrivateKey)
 * @param mnemonic
 */
module.exports = function fromMnemonic(mnemonic) {
  if (!is.mnemonic(mnemonic)) {
    throw new Error('Expected a valid mnemonic (typeof String or Mnemonic)');
  }
  const trimmedMnemonic = mnemonic.toString().trim();
  this.walletType = WALLET_TYPES.HDWALLET;
  this.mnemonic = trimmedMnemonic; // todo : What about without this ?
  this.HDPrivateKey = mnemonicToHDPrivateKey(trimmedMnemonic, this.network, this.passphrase);
  this.keyChain = new KeyChain({ HDPrivateKey: this.HDPrivateKey });
};
