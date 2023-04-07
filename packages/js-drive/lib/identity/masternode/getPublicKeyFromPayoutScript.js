/**
 * @typedef getPublicKeyFromPayoutScript
 * @param {Script} payoutScript
 * @param {number} type
 * @param {WebAssembly.Instance} dppWasm
 * @returns {Buffer}
 */
function getPublicKeyFromPayoutScript(payoutScript, type, dppWasm) {
  switch (type) {
    case dppWasm.IdentityPublicKey.TYPES.ECDSA_HASH160:
      return payoutScript.toBuffer().slice(3, 23);
    case dppWasm.IdentityPublicKey.TYPES.BIP13_SCRIPT_HASH:
      return payoutScript.toBuffer().slice(2, 22);
    default: {
      throw new dppWasm.InvalidIdentityPublicKeyTypeError(type);
    }
  }
}

module.exports = getPublicKeyFromPayoutScript;
