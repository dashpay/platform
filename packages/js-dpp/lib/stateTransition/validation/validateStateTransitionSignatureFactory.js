const ValidationResult = require('../../validation/ValidationResult');
const IdentityPublicKey = require('../../identity/IdentityPublicKey');
const InvalidIdentityPublicKeyTypeError = require('../../errors/InvalidIdentityPublicKeyTypeError');
const InvalidStateTransitionSignatureError = require('../../errors/InvalidStateTransitionSignatureError');
const MissingPublicKeyError = require('../../errors/MissingPublicKeyError');

/**
 * Validate state transition signature
 *
 * @param {DataProvider} dataProvider
 * @returns {validateStateTransitionSignature}
 */
function validateStateTransitionSignatureFactory(dataProvider) {
  /**
   * @typedef validateStateTransitionSignature
   * @param {
   * DataContractStateTransition|
   * DocumentsStateTransition
   * } stateTransition
   * @param {string} userId
   * @returns {Promise<ValidationResult>}
   */
  async function validateStateTransitionSignature(stateTransition, userId) {
    const result = new ValidationResult();

    const identity = await dataProvider.fetchIdentity(userId);

    const publicKey = identity.getPublicKeyById(stateTransition.getSignaturePublicKeyId());

    if (!publicKey) {
      result.addError(
        new MissingPublicKeyError(stateTransition.getSignaturePublicKeyId()),
      );

      return result;
    }

    if (publicKey.getType() !== IdentityPublicKey.TYPES.ECDSA_SECP256K1) {
      result.addError(
        new InvalidIdentityPublicKeyTypeError(publicKey.getType()),
      );

      return result;
    }

    const signatureIsValid = stateTransition.verifySignature(publicKey);

    if (!signatureIsValid) {
      result.addError(
        new InvalidStateTransitionSignatureError(stateTransition),
      );
    }

    return result;
  }

  return validateStateTransitionSignature;
}

module.exports = validateStateTransitionSignatureFactory;
