const InvalidStateTransitionSignatureError = require('../../errors/consensus/signature/InvalidStateTransitionSignatureError');

const ValidationResult = require('../../validation/ValidationResult');
const SignatureVerificationOperation = require('../fee/operations/SignatureVerificationOperation');
const { TYPES } = require('../../identity/IdentityPublicKey');
const StateTransitionExecutionContext = require('../StateTransitionExecutionContext');
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

      // We use temporary execution context without dry run,
      // because despite the dryRun, we need to get the
      // identity to proceed with following logic
      const tmpExecutionContext = new StateTransitionExecutionContext();

      // Target identity must exist
      const identity = await stateRepository.fetchIdentity(targetIdentityId, tmpExecutionContext);

      // Collect operations back from temporary context
      executionContext.addOperation(...tmpExecutionContext.getOperations());

      if (!identity) {
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
