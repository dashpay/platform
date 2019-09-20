/**
 * To any object passed (Transaction, ST,..), will try to sign the message given passed keys.
 * @param object {Transaction} - The object to sign
 * @param privateKeys
 * @param sigType
 */
module.exports = function sign(object, privateKeys, sigType) {
  return this.keyChain.sign(object, privateKeys, sigType);
};
