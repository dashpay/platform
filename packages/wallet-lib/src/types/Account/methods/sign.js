const { PrivateKey, HDPrivateKey } = require('@dashevo/dashcore-lib');
/**
 * To any object passed (Transaction, ST,..), will try to sign the message given passed keys.
 * @param {Transaction} object - The object to sign
 * @param {[PrivateKey]} privateKeys - A set of private keys to sign the inputs with
 * @param {number} [sigType] - a valid signature value (Dashcore.Signature)
 * @return {Transaction} transaction - the signed transaction
 */
module.exports = function sign(object, privateKeys = [], sigType) {
  const { network } = this;

  if (object.inputs && (!privateKeys || !privateKeys.length)) {
    const addressList = [];
    // We seek private key based on inputs
    object.inputs.forEach((input) => {
      if (input.script) {
        // eslint-disable-next-line no-underscore-dangle
        const addr = input.script.toAddress(network) || input.output._script.toAddress(network);
        addressList.push(addr.toString());
      }
    });
    this.getPrivateKeys(addressList).forEach((pk) => {
      if (pk.constructor.name === PrivateKey.name) {
        privateKeys.push(pk);
      } else if (pk.constructor.name === HDPrivateKey.name) {
        privateKeys.push(pk.privateKey);
      } else {
        throw new Error(`Unexpected pk of type ${pk.constructor.name}`);
      }
    });
  }

  return this.keyChain.sign(object, privateKeys, sigType);
};
