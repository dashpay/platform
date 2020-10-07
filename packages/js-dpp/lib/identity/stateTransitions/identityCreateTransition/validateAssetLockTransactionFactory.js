const { Signer: { verifyHashSignature } } = require('@dashevo/dashcore-lib');
const ValidationResult = require('../../../validation/ValidationResult');

const ConsensusError = require('../../../errors/ConsensusError');
const InvalidIdentityAssetLockTransactionOutputError = require('../../../errors/InvalidIdentityAssetLockTransactionOutputError');
const InvalidStateTransitionSignatureError = require('../../../errors/InvalidStateTransitionSignatureError');

/**
 *
 * @param {fetchConfirmedAssetLockTransactionOutput} fetchConfirmedAssetLockTransactionOutput
 * @return {validateAssetLockTransaction}
 */
function validateAssetLockTransactionFactory(fetchConfirmedAssetLockTransactionOutput) {
  /**
   * Validates identityCreateTransition signature against lock transaction
   *
   * @typedef validateAssetLockTransaction
   * @param {IdentityCreateTransition|IdentityTopUpTransition} identityStateTransition
   * @returns {Promise<ValidationResult>}
   */
  async function validateAssetLockTransaction(identityStateTransition) {
    // fetch lock transaction output, extract pubkey from it and verify signature

    const result = new ValidationResult();

    let output;

    try {
      output = await fetchConfirmedAssetLockTransactionOutput(
        identityStateTransition.getLockedOutPoint().toString(),
      );
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
      result.addError(
        new InvalidIdentityAssetLockTransactionOutputError('Output is not a valid standard OP_RETURN output', output),
      );
    }

    const publicKeyHash = script.getData();

    if (publicKeyHash.length !== 20) {
      result.addError(
        new InvalidIdentityAssetLockTransactionOutputError('Output has invalid public key hash', output),
      );
    }

    if (!result.isValid()) {
      return result;
    }

    let signatureIsVerified;

    try {
      signatureIsVerified = verifyHashSignature(
        Buffer.from(identityStateTransition.hash({ skipSignature: true }), 'hex'),
        identityStateTransition.getSignature(),
        publicKeyHash,
      );
    } catch (e) {
      signatureIsVerified = false;
    }

    if (!signatureIsVerified) {
      result.addError(new InvalidStateTransitionSignatureError(identityStateTransition));
    }

    return result;
  }

  return validateAssetLockTransaction;
}

module.exports = validateAssetLockTransactionFactory;
