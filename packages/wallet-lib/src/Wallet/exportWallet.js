const { WALLET_TYPES } = require('../CONSTANTS');

/**
 * Export the wallet (mnemonic)
 * @param toHDPrivateKey - Default: false - Allow to return to a HDPrivateKey type
 * @return {Mnemonic|HDPrivateKey}
 */
module.exports = function exportWallet(toHDPrivateKey = false) {
  function exportMnemonic(mnemonic) {
    if (!mnemonic) throw new Error('Wallet was not initiated with a mnemonic, can\'t export it');
    return mnemonic.toString();
  }

  if (toHDPrivateKey) {
    return this.HDPrivateKey;
  }
  switch (this.type) {
    case WALLET_TYPES.SINGLE_ADDRESS:
      if (!this.privateKey) throw new Error('No privateKey to export');
      return this.privateKey;
    case WALLET_TYPES.HDWALLET:
      return exportMnemonic(this.mnemonic);
    default:
      throw new Error('Trying to export from an unknown wallet type');
  }
};
