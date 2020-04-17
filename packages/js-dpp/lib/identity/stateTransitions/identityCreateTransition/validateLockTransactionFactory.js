const { Signer: { verifyHashSignature } } = require('@dashevo/dashcore-lib');
const ValidationResult = require('../../../validation/ValidationResult');

const ConsensusError = require('../../../errors/ConsensusError');
const InvalidIdentityLockTransactionOutputError = require('../../../errors/InvalidIdentityLockTransactionOutputError');
const InvalidStateTransitionSignatureError = require('../../../errors/InvalidStateTransitionSignatureError');
/**
 *
 * @param {getLockedTransactionOutput} getLockedTransactionOutput
 * @return {validateLockTransaction}
 */
function validateLockTransactionFactory(getLockedTransactionOutput) {
  /**
   * Validates identityCreateTransition signature against lock transaction
   *
   * @typedef validateLockTransaction
   * @param {IdentityCreateTransition} identityCreateTransition
   * @returns {Promise<ValidationResult>}
   */
  async function validateLockTransaction(identityCreateTransition) {
    // fetch lock transaction output, extract pubkey from it and verify signature

    const result = new ValidationResult();

    let output;

    try {
      output = await getLockedTransactionOutput(identityCreateTransition.getLockedOutPoint());
    } catch (e) {
      if (e instanceof ConsensusError) {
        result.addError(e);
      } else {
        throw e;
      }
    }

    if (!result.isValid()) {
      return result;
    }

    const { script } = output;

    if (!script.isDataOut()) {
      result.addError(new InvalidIdentityLockTransactionOutputError('Output is not a valid standard OP_RETURN output', output));
    }

    const publicKeyHash = script.getData();

    if (publicKeyHash.length !== 20) {
      result.addError(new InvalidIdentityLockTransactionOutputError('Output has invalid public key hash', output));
    }

    if (!result.isValid()) {
      return result;
    }

    let signatureIsVerified;

    try {
      signatureIsVerified = verifyHashSignature(
        Buffer.from(identityCreateTransition.hash({ skipSignature: true }), 'hex'),
        Buffer.from(identityCreateTransition.getSignature(), 'base64'),
        publicKeyHash,
      );
    } catch (e) {
      signatureIsVerified = false;
    }

    if (!signatureIsVerified) {
      result.addError(new InvalidStateTransitionSignatureError(identityCreateTransition));
    }

    return result;
  }

  return validateLockTransaction;
}

module.exports = validateLockTransactionFactory;
