const InvalidStateTransitionSignatureError = require('../../errors/InvalidStateTransitionSignatureError');

const ValidationResult = require('../../validation/ValidationResult');

/**
 * @param {createStateTransition} createStateTransition
 * @param {Function} verifyHashSignature
 * @returns {validateSignatureAgainstAssetLockPublicKey}
 */
function validateSignatureAgainstAssetLockPublicKeyFactory(
  createStateTransition,
  verifyHashSignature,
) {
  /**
   * @typedef {validateSignatureAgainstAssetLockPublicKey}
   * @param {RawStateTransition} rawStateTransition
   * @param {Buffer} publicKeyHash
   * @returns {Promise<ValidationResult>}
   */
  async function validateSignatureAgainstAssetLockPublicKey(rawStateTransition, publicKeyHash) {
    const result = new ValidationResult();

    const stateTransition = await createStateTransition(rawStateTransition);
    const stateTransitionHash = stateTransition.hash({ skipSignature: true });


    let signatureIsVerified;

    try {
      signatureIsVerified = verifyHashSignature(
        stateTransitionHash,
        stateTransition.getSignature(),
        publicKeyHash,
      );
    } catch (e) {
      signatureIsVerified = false;
    }

    if (!signatureIsVerified) {
      result.addError(new InvalidStateTransitionSignatureError(stateTransition));
    }

    return result;
  }

  return validateSignatureAgainstAssetLockPublicKey;
}

module.exports = validateSignatureAgainstAssetLockPublicKeyFactory;
