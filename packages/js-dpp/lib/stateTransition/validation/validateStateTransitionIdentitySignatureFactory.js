const IdentityPublicKey = require('../../identity/IdentityPublicKey');
const InvalidIdentityPublicKeyTypeError = require('../../errors/consensus/signature/InvalidIdentityPublicKeyTypeError');
const InvalidStateTransitionSignatureError = require('../../errors/consensus/signature/InvalidStateTransitionSignatureError');
const MissingPublicKeyError = require('../../errors/consensus/signature/MissingPublicKeyError');
const PublicKeySecurityLevelNotMetError = require('../errors/PublicKeySecurityLevelNotMetError');
const WrongPublicKeyPurposeError = require('../errors/WrongPublicKeyPurposeError');
const PublicKeyIsDisabledError = require('../errors/PublicKeyIsDisabledError');
const Script = require('@dashevo/dashcore-lib/lib/script');
const InvalidSignatureScriptError = require('../../errors/consensus/signature/InvalidSignatureScriptError');

/**
 * Validate state transition signature
 *
 * @param {validateIdentityExistence} validateIdentityExistence
 * @returns {validateStateTransitionIdentitySignature}
 */
function validateStateTransitionIdentitySignatureFactory(
  validateIdentityExistence,
) {
  /**
   * @typedef validateStateTransitionIdentitySignature
   * @param {
   * DataContractCreateTransition|
   * DocumentsBatchTransition
   * } stateTransition
   * @returns {Promise<ValidationResult>}
   */
  async function validateStateTransitionIdentitySignature(stateTransition) {
    // Owner must exist
    const result = await validateIdentityExistence(stateTransition.getOwnerId());

    if (!result.isValid()) {
      return result;
    }

    // Signature must be valid
    const identity = result.getData();

    const publicKey = identity.getPublicKeyById(stateTransition.getSignaturePublicKeyId());

    if (!publicKey) {
      result.addError(
        new MissingPublicKeyError(stateTransition.getSignaturePublicKeyId()),
      );

      return result;
    }

    try {
      stateTransition.verifyPublicKeyLevelAndPurpose(publicKey);
    } catch (e) {
      // TODO PublicKeySecurityLevelNotMetError

      // TODO WrongPublicKeyPurposeError
    }

    try {
      stateTransition.verifyPublicKeyIsEnabled(publicKey);
    } catch (e) {
      // TODO PublicKeyIsDisabledError
    }

    if (publicKey.getType() === IdentityPublicKey.TYPES.BIP13_SCRIPT_HASH) {
      const rawSignatureScript = stateTransition.getSignatureScript();

      if (!rawSignatureScript || rawSignatureScript.length === 0) {
        // TODO Not present


        return result;
      }

      let signatureScript;
      try {
        signatureScript = new Script(rawSignatureScript);
      } catch (e) {
        result.addError(
          new InvalidSignatureScriptError(rawSignatureScript),
        )
      }

      if (!result.isValid()) {
        return result;
      }

      const address = signatureScript.toAddress();

      if (!address || !address.isPayToScriptHash()) {
        result.addError(
          new InvalidSignatureScriptError(rawSignatureScript),
        );
      }
    } else {
      const signature = stateTransition.getSignature();

      if (!signature || signature.length === 0) {
        // TODO Not present
      }
    }

    const signatureIsValid = await stateTransition.verifySignature(publicKey);

    if (!signatureIsValid) {
      result.addError(
        new InvalidStateTransitionSignatureError(stateTransition),
      );
    }

    return result;
  }

  return validateStateTransitionIdentitySignature;
}

module.exports = validateStateTransitionIdentitySignatureFactory;
