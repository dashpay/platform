const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const InvalidIdentityPublicKeyTypeError = require('@dashevo/dpp/lib/stateTransition/errors/InvalidIdentityPublicKeyTypeError');

/**
 * @typedef getPublicKeyFromPayoutScript
 * @param {Buffer} payoutScriptBuffer
 * @param {number} type
 * @returns {Buffer}
 */
function getPublicKeyFromPayoutScript(payoutScriptBuffer, type) {
  switch (type) {
    case IdentityPublicKey.TYPES.ECDSA_HASH160:
      return payoutScriptBuffer.slice(3, 23);
    case IdentityPublicKey.TYPES.BIP13_SCRIPT_HASH:
      return payoutScriptBuffer.slice(2, 22);
    default:
      throw new InvalidIdentityPublicKeyTypeError(type);
  }
}

module.exports = getPublicKeyFromPayoutScript;
