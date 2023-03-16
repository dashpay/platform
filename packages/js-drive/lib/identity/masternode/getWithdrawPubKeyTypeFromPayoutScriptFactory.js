const InvalidPayoutScriptError = require('./errors/InvalidPayoutScriptError');

/**
 *
 * @param {string} network
 * @param {WebAssembly.Instance} dppWasm
 * @returns {getWithdrawPubKeyTypeFromPayoutScript}
 */
function getWithdrawPubKeyTypeFromPayoutScriptFactory(network, dppWasm) {
  /**
   * @typedef getWithdrawPubKeyTypeFromPayoutScript
   * @param {Script} payoutScript
   * @returns {number}
   */
  function getWithdrawPubKeyTypeFromPayoutScript(payoutScript) {
    const address = payoutScript.toAddress(network);

    if (address === false) {
      throw new InvalidPayoutScriptError(payoutScript);
    }

    let withdrawPubKeyType;
    if (address.isPayToScriptHash()) {
      withdrawPubKeyType = dppWasm.KeyType.BIP13_SCRIPT_HASH;
    } else if (address.isPayToPublicKeyHash()) {
      withdrawPubKeyType = dppWasm.KeyType.ECDSA_HASH160;
    } else {
      throw new InvalidPayoutScriptError(payoutScript);
    }

    return withdrawPubKeyType;
  }

  return getWithdrawPubKeyTypeFromPayoutScript;
}

module.exports = getWithdrawPubKeyTypeFromPayoutScriptFactory;
