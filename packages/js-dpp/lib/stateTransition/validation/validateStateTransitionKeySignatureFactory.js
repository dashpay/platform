const InvalidStateTransitionSignatureError = require('../../errors/consensus/signature/InvalidStateTransitionSignatureError');

const ValidationResult = require('../../validation/ValidationResult');
const SignatureVerificationOperation = require('../fee/operations/SignatureVerificationOperation');
const { TYPES } = require('../../identity/IdentityPublicKey');
const IdentityNotFoundError = require('../../errors/consensus/signature/IdentityNotFoundError');
const stateTransitionTypes = require('../stateTransitionTypes');

/**
 * @param {Function} verifyHashSignature
 * @param {fetchAssetLockPublicKeyHash} fetchAssetLockPublicKeyHash
 * @param {StateRepository} stateRepository
 * @returns {validateStateTransitionKeySignature}
 */
function validateStateTransitionKeySignatureFactory(
  verifyHashSignature,
  fetchAssetLockPublicKeyHash,
  stateRepository,
) {
  /**
   * @typedef {validateStateTransitionKeySignature}
   * @param {IdentityCreateTransition|IdentityTopUpTransition} stateTransition
   * @returns {Promise<ValidationResult>}
   */
  async function validateStateTransitionKeySignature(stateTransition) {
    const result = new ValidationResult();

    const executionContext = stateTransition.getExecutionContext();

    // Validate target identity existence for top up
    if (stateTransition.getType() === stateTransitionTypes.IDENTITY_TOP_UP) {
      const targetIdentityId = stateTransition.getIdentityId();

      // Target identity must exist
      const identityBalance = await stateRepository.fetchIdentityBalance(
        targetIdentityId,
        executionContext,
      );

      if (identityBalance === null) {
        result.addError(new IdentityNotFoundError(targetIdentityId.toBuffer()));

        return result;
      }
    }

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
