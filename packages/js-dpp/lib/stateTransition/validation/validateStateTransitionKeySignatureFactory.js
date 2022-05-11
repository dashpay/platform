const InvalidStateTransitionSignatureError = require('../../errors/consensus/signature/InvalidStateTransitionSignatureError');

const ValidationResult = require('../../validation/ValidationResult');
const SignatureVerificationOperation = require('../fee/operations/SignatureVerificationOperation');
const { TYPES } = require('../../identity/IdentityPublicKey');

/**
 * @param {Function} verifyHashSignature
 * @param {fetchAssetLockPublicKeyHash} fetchAssetLockPublicKeyHash
 * @returns {validateStateTransitionKeySignature}
 */
function validateStateTransitionKeySignatureFactory(
  verifyHashSignature,
  fetchAssetLockPublicKeyHash,
) {
  /**
   * @typedef {validateStateTransitionKeySignature}
   * @param {IdentityCreateTransition|IdentityTopUpTransition} stateTransition
   * @returns {Promise<ValidationResult>}
   */
  async function validateStateTransitionKeySignature(stateTransition) {
    const result = new ValidationResult();

    const executionContext = stateTransition.getExecutionContext();

    const stateTransitionHash = stateTransition.hash({ skipSignature: true });

    const publicKeyHash = await fetchAssetLockPublicKeyHash(
      stateTransition.getAssetLockProof(),
      executionContext,
    );

    const operation = new SignatureVerificationOperation(TYPES.ECDSA_SECP256K1);

    executionContext.addOperation(operation);

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
      result.addError(new InvalidStateTransitionSignatureError());
    }

    return result;
  }

  return validateStateTransitionKeySignature;
}

module.exports = validateStateTransitionKeySignatureFactory;
