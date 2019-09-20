const {
  crypto, Transaction,
} = require('@dashevo/dashcore-lib');

/**
 * Allow to sign any transaction or a transition object from a valid privateKeys list
 * @param object
 * @param privateKeys
 * @param sigType
 */
function sign(object, privateKeys, sigType = crypto.Signature.SIGHASH_ALL) {
  const handledTypes = [Transaction.name, Transaction.Payload.SubTxRegisterPayload];
  if (!privateKeys) throw new Error('Require one or multiple privateKeys to sign');
  if (!object) throw new Error('Nothing to sign');
  if (!handledTypes.includes(object.constructor.name)) {
    throw new Error(`Keychain sign : Unhandled object of type ${object.constructor.name}`);
  }
  const obj = object.sign(privateKeys, sigType);

  if (!obj.isFullySigned()) {
    throw new Error('Not fully signed transaction');
  }
  return obj;
}
module.exports = sign;
