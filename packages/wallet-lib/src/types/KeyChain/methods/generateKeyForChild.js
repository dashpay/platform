const {
  HDPublicKey,
} = require('@dashevo/dashcore-lib');
/**
 * Derive from HDPrivateKey to a child
 * @param {number} index - Child index to derivee to
 * @param {HDPrivateKey|HDPublicKey} [type=HDPrivateKey] - set the type of returned keys
 * @return {HDPrivateKey|HDPublicKey}
 */
function generateKeyForChild(index, type = 'HDPrivateKey') {
  if (!['HDPrivateKey', 'HDPublicKey'].includes(this.type)) {
    throw new Error('Wallet is not loaded from a mnemonic or a HDPubKey, impossible to derivate child');
  }
  const HDKey = this[this.type];
  const hdPublicKey = HDKey.deriveChild(index);
  if (type === 'HDPublicKey') return HDPublicKey(hdPublicKey);
  return hdPublicKey;
}
module.exports = generateKeyForChild;
