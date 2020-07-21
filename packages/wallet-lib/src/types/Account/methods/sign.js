/**
 * To any object passed (Transaction, ST,..), will try to sign the message given passed keys.
 * @param {Transaction} object - The object to sign
 * @param {[PrivateKey]} privateKeys - A set of private keys to sign the inputs with
 * @param {number} [sigType] - a valid signature value (Dashcore.Signature)
 * @return {Transaction} transaction - the signed transaction
 */
module.exports = function sign(object, privateKeys, sigType) {
  return this.keyChain.sign(object, privateKeys, sigType);
};
