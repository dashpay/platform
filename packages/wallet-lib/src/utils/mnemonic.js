const { pbkdf2Sync } = require('pbkdf2');
const { Mnemonic, HDPrivateKey } = require('@dashevo/dashcore-lib');
const { doubleSha256 } = require('./crypto');

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
function mnemonicToHDPrivateKey(mnemonic, network = 'testnet', passphrase = '') {
  if (!mnemonic) throw new Error('Expect mnemonic to be provided');

  return (mnemonic.constructor.name === Mnemonic.name)
    ? mnemonic.toHDPrivateKey(passphrase, network)
    : new Mnemonic(mnemonic).toHDPrivateKey(passphrase, network);
}

function mnemonicToWalletId(mnemonic) {
  if (!mnemonic) throw new Error('Expect mnemonic to be provided');

  const buffMnemonic = Buffer.from(mnemonic.toString());
  const buff = doubleSha256(buffMnemonic);
  return buff.toString('hex').slice(0, 10);
}
const mnemonicToSeed = function mnemonicToSeed(mnemonic, password = '') {
  const mnemonicBuff = Buffer.from(mnemonic.normalize('NFKD'), 'utf8');
  const saltBuff = Buffer.from(`mnemonic${password}`, 'utf8');
  return pbkdf2Sync(mnemonicBuff, saltBuff, 2048, 64, 'sha512')
    .toString('hex');
};
// See https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki
const seedToHDPrivateKey = function seedToHDPrivateKey(seed, network = 'testnet') {
  return HDPrivateKey.fromSeed(seed, network);
};
module.exports = {
  generateNewMnemonic,
  mnemonicToHDPrivateKey,
  mnemonicToWalletId,
  mnemonicToSeed,
  seedToHDPrivateKey,
};
