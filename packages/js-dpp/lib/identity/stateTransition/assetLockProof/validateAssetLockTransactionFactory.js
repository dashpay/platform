const { Transaction } = require('@dashevo/dashcore-lib');

const InvalidIdentityAssetLockTransactionError = require('../../../errors/consensus/basic/identity/InvalidIdentityAssetLockTransactionError');
const IdentityAssetLockTransactionOutputNotFoundError = require('../../../errors/consensus/basic/identity/IdentityAssetLockTransactionOutputNotFoundError');
const InvalidIdentityAssetLockTransactionOutputError = require('../../../errors/consensus/basic/identity/InvalidIdentityAssetLockTransactionOutputError');
const ValidationResult = require('../../../validation/ValidationResult');
const IdentityAssetLockTransactionOutPointAlreadyExistsError = require('../../../errors/consensus/basic/identity/IdentityAssetLockTransactionOutPointAlreadyExistsError');
const InvalidAssetLockTransactionOutputReturnSize = require('../../../errors/consensus/basic/identity/InvalidAssetLockTransactionOutputReturnSize');

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
    } catch (error) {
      const consensusError = new InvalidIdentityAssetLockTransactionError(error.message);

      consensusError.setValidationError(error);

      result.addError(consensusError);

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
        new InvalidIdentityAssetLockTransactionOutputError(outputIndex),
      );

      return result;
    }

    const publicKeyHash = output.script.getData();

    if (publicKeyHash.length !== 20) {
      result.addError(
        new InvalidAssetLockTransactionOutputReturnSize(outputIndex),
      );

      return result;
    }

    const outPointBuffer = transaction.getOutPointBuffer(outputIndex);
    const outPointIsUsed = await stateRepository.isAssetLockTransactionOutPointAlreadyUsed(
      outPointBuffer,
    );

    if (outPointIsUsed) {
      result.addError(
        new IdentityAssetLockTransactionOutPointAlreadyExistsError(
          Buffer.from(transaction.id, 'hex'),
          outputIndex,
        ),
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
