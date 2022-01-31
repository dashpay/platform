const {
  PrivateKey,
} = require('@dashevo/dashcore-lib');

/**
 * @return {PrivateKey}
 */
function getPrivateKey() {
  let pk;
  if (this.type === 'HDPrivateKey') {
    pk = PrivateKey(this.HDPrivateKey.privateKey);
  }
  if (this.type === 'privateKey') {
    pk = PrivateKey(this.privateKey);
  }
  return pk;
}
module.exports = getPrivateKey;
