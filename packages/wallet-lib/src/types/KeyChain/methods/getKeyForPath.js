const { HDPrivateKey } = require('@dashevo/dashcore-lib');
/**
 * Get a key from the cache or generate if none
 * @param path
 * @param type - def : HDPrivateKey - Expected return datatype of the keys
 * @return {HDPrivateKey | HDPublicKey}
 */
function getKeyForPath(path, type = 'HDPrivateKey') {
  if (type === 'HDPublicKey') {
    // In this case, we do not generate or keep in cache.
    return this.generateKeyForPath(path, type);
  }
  if (!this.keys[path]) {
    if (this.type === 'HDPrivateKey') {
      this.keys[path] = this.generateKeyForPath(path, type).toString();
    }
    if (this.type === 'privateKey') {
      this.keys[path] = this.getPrivateKey(path).toString();
    }
  }

  return new HDPrivateKey(this.keys[path]);
}
module.exports = getKeyForPath;
