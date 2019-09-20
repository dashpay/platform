const {
  HDPublicKey,
} = require('@dashevo/dashcore-lib');
/**
 * Derive from HDPrivateKey to a specific path
 * @param path
 * @param type - {HDPrivateKey|HDPublicKey} def : HDPrivateKey - set the type of returned keys
 * @return HDPrivateKey
 */
function generateKeyForPath(path, type = 'HDPrivateKey') {
  if (!['HDPrivateKey', 'HDPublicKey'].includes(this.type)) {
    throw new Error('Wallet is not loaded from a mnemonic or a HDPubKey, impossible to derivate keys');
  }
  const HDKey = this[this.type];
  const hdPrivateKey = HDKey.derive(path);
  if (type === 'HDPublicKey') return HDPublicKey(hdPrivateKey);
  return hdPrivateKey;
}
module.exports = generateKeyForPath;
