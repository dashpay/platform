const { Transaction } = require('@dashevo/dashcore-lib');

const InvalidIdentityAssetLockTransactionError = require('../../../errors/InvalidIdentityAssetLockTransactionError');
const IdentityAssetLockTransactionOutputNotFoundError = require('../../../errors/IdentityAssetLockTransactionOutputNotFoundError');
const InvalidIdentityAssetLockTransactionOutputError = require('../../../errors/InvalidIdentityAssetLockTransactionOutputError');
const ValidationResult = require('../../../validation/ValidationResult');
const IdentityAssetLockTransactionOutPointAlreadyExistsError = require('../../../errors/IdentityAssetLockTransactionOutPointAlreadyExistsError');

/**
 *
 * @param {StateRepository} stateRepository
 * @returns {validateAssetLockTransaction}
 */
function validateAssetLockTransactionFactory(stateRepository) {
  /**
   * @typedef validateAssetLockTransaction
   * @param {Buffer} rawTransaction
   * @param {number} outputIndex
   * @returns {Promise<ValidationResult>}
   */

  async function validateAssetLockTransaction(rawTransaction, outputIndex) {
    const result = new ValidationResult();
    /**
     * @type {Transaction}
     */
    let transaction;
    try {
      transaction = new Transaction(rawTransaction);
    } catch (e) {
      const error = new InvalidIdentityAssetLockTransactionError(e.message);

      result.addError(error);

      return result;
    }

    const output = transaction.outputs[outputIndex];

    if (!output) {
      result.addError(
        new IdentityAssetLockTransactionOutputNotFoundError(outputIndex),
      );

      return result;
    }

    if (!output.script.isDataOut()) {
      result.addError(
        new InvalidIdentityAssetLockTransactionOutputError('Output is not a valid standard OP_RETURN output', output),
      );

      return result;
    }

    const publicKeyHash = output.script.getData();

    if (publicKeyHash.length !== 20) {
      result.addError(
        new InvalidIdentityAssetLockTransactionOutputError('Output has invalid public key hash', output),
      );

      return result;
    }

    const outPointBuffer = transaction.getOutPointBuffer(outputIndex);
    const outPointIsUsed = await stateRepository.isAssetLockTransactionOutPointAlreadyUsed(
      outPointBuffer,
    );

    if (outPointIsUsed) {
      result.addError(
        new IdentityAssetLockTransactionOutPointAlreadyExistsError(outPointBuffer),
      );

      return result;
    }

    result.setData({
      publicKeyHash,
      transaction,
    });

    return result;
  }

  return validateAssetLockTransaction;
}

module.exports = validateAssetLockTransactionFactory;
