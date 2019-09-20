/**
 * Generate a key by deriving it's direct child
 * @param index - {Number}
 * @return {HDPrivateKey | HDExtPublicKey}
 */
function getKeyForChild(index = 0, type = 'HDPrivateKey') {
  return this.generateKeyForChild(index, type);
}

module.exports = getKeyForChild;
