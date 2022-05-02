const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const Script = require('@dashevo/dashcore-lib/lib/script');

const InvalidPayoutScriptError = require('./errors/InvalidPayoutScriptError');

/**
 *
 * @param {string} network
 * @returns {getWithdrawPubKeyTypeFromPayoutScript}
 */
function getWithdrawPubKeyTypeFromPayoutScriptFactory(network) {
  /**
   * @typedef getWithdrawPubKeyTypeFromPayoutScript
   * @param {string|Buffer} payoutScript
   * @returns {number}
   */
  function getWithdrawPubKeyTypeFromPayoutScript(payoutScript) {
    const address = new Script(payoutScript).toAddress(network);

    if (address === false) {
      throw new InvalidPayoutScriptError(payoutScript);
    }

    let withdrawPubKeyType;
    if (address.isPayToScriptHash()) {
      withdrawPubKeyType = IdentityPublicKey.TYPES.BIP13_SCRIPT_HASH;
    } else if (address.isPayToPublicKeyHash()) {
      withdrawPubKeyType = IdentityPublicKey.TYPES.ECDSA_HASH160;
    } else {
      throw new InvalidPayoutScriptError(payoutScript);
    }

    return withdrawPubKeyType;
  }

  return getWithdrawPubKeyTypeFromPayoutScript;
}

module.exports = getWithdrawPubKeyTypeFromPayoutScriptFactory;
