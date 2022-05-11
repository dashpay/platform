const IdentityPublicKey = require('../../identity/IdentityPublicKey');
const InvalidIdentityPublicKeyTypeError = require('../../errors/consensus/signature/InvalidIdentityPublicKeyTypeError');
const InvalidStateTransitionSignatureError = require('../../errors/consensus/signature/InvalidStateTransitionSignatureError');
const MissingPublicKeyError = require('../../errors/consensus/signature/MissingPublicKeyError');
const VerifySignatureOperation = require('../fee/operations/VerifySignatureOperation');

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
    const executionContext = stateTransition.getExecutionContext();

    // Owner must exist
    const result = await validateIdentityExistence(
      stateTransition.getOwnerId(),
      executionContext,
    );

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

    if (
      publicKey.getType() !== IdentityPublicKey.TYPES.ECDSA_SECP256K1
      && publicKey.getType() !== IdentityPublicKey.TYPES.ECDSA_HASH160
    ) {
      result.addError(
        new InvalidIdentityPublicKeyTypeError(publicKey.getType()),
      );

      return result;
    }

    const operation = new VerifySignatureOperation(publicKey.getType());

    executionContext.addOperation(operation);

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
