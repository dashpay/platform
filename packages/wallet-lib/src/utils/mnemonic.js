const Mnemonic = require('@dashevo/dashcore-mnemonic');
const { hash256 } = require('./crypto');

function generateNewMnemonic() {
  return Mnemonic();
}

/**
 * Will return the HDPrivateKey from a Mnemonic
 * @param {Mnemonic|String} mnemonic
 * @param {Networks | String} network
 * @param {String} passphrase
 * @return {HDPrivateKey}
 */
function mnemonicToHDPrivateKey(mnemonic, network, passphrase = '') {
  if (!mnemonic) throw new Error('Expect mnemonic to be provided');

  return (mnemonic.constructor.name === 'Mnemonic')
    ? mnemonic.toHDPrivateKey(passphrase, network)
    : new Mnemonic(mnemonic).toHDPrivateKey(passphrase, network);
}

function mnemonicToWalletId(mnemonic) {
  if (!mnemonic) throw new Error('Expect mnemonic to be provided');

  const buffMnemonic = Buffer.from(mnemonic.toString());
  const buff = hash256(buffMnemonic);
  return buff.toString('hex').slice(0, 10);
}

module.exports = {
  generateNewMnemonic,
  mnemonicToHDPrivateKey,
  mnemonicToWalletId,
};
