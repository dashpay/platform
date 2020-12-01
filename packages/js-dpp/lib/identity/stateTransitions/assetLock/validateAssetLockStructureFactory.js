const { Transaction } = require('@dashevo/dashcore-lib');

const assetLockSchema = require('../../../../schema/identity/stateTransition/assetLock/assetLock.json');

const convertBuffersToArrays = require('../../../util/convertBuffersToArrays');
const InvalidIdentityAssetLockTransactionError = require('../../../errors/InvalidIdentityAssetLockTransactionError');
const IdentityAssetLockTransactionOutputNotFoundError = require('../../../errors/IdentityAssetLockTransactionOutputNotFoundError');
const InvalidIdentityAssetLockTransactionOutputError = require('../../../errors/InvalidIdentityAssetLockTransactionOutputError');
const IdentityAssetLockTransactionOutPointAlreadyExistsError = require('../../../errors/IdentityAssetLockTransactionOutPointAlreadyExistsError');

/**
 * @param {JsonSchemaValidator} jsonSchemaValidator
 * @param {Object.<number, Function>} proofValidationFunctionsByType
 * @param {StateRepository} stateRepository
 * @returns {validateAssetLockStructure}
 */
function validateAssetLockStructureFactory(
  jsonSchemaValidator,
  proofValidationFunctionsByType,
  stateRepository,
) {
  /**
   * @typedef {validateAssetLockStructure}
   * @param {RawAssetLock} rawAssetLock
   * @returns {Promise<ValidationResult>}
   */
  async function validateAssetLockStructure(rawAssetLock) {
    const result = jsonSchemaValidator.validate(
      assetLockSchema,
      convertBuffersToArrays(rawAssetLock),
    );

    if (!result.isValid()) {
      return result;
    }

    /**
     * @type {Transaction}
     */
    let transaction;
    try {
      transaction = new Transaction(rawAssetLock.transaction);
    } catch (e) {
      const error = new InvalidIdentityAssetLockTransactionError(e.message);

      result.addError(error);

      return result;
    }

    if (!transaction.outputs[rawAssetLock.outputIndex]) {
      result.addError(
        new IdentityAssetLockTransactionOutputNotFoundError(rawAssetLock.outputIndex),
      );

      return result;
    }

    if (!result.isValid()) {
      return result;
    }

    const output = transaction.outputs[rawAssetLock.outputIndex];

    if (!output.script.isDataOut()) {
      result.addError(
        new InvalidIdentityAssetLockTransactionOutputError('Output is not a valid standard OP_RETURN output', output),
      );
    }

    const publicKeyHash = output.script.getData();

    if (publicKeyHash.length !== 20) {
      result.addError(
        new InvalidIdentityAssetLockTransactionOutputError('Output has invalid public key hash', output),
      );
    }

    if (!result.isValid()) {
      return result;
    }

    const proofValidationFunction = proofValidationFunctionsByType[rawAssetLock.proof.type];

    result.merge(
      await proofValidationFunction(rawAssetLock, transaction),
    );

    if (!result.isValid()) {
      return result;
    }

    const outPointBuffer = transaction.getOutPointBuffer(rawAssetLock.outputIndex);
    const outPointExists = await stateRepository.checkAssetLockTransactionOutPointExists(
      outPointBuffer,
    );

    if (outPointExists) {
      result.addError(
        new IdentityAssetLockTransactionOutPointAlreadyExistsError(outPointBuffer),
      );
    }

    if (result.isValid()) {
      result.setData(publicKeyHash);
    }

    return result;
  }

  return validateAssetLockStructure;
}

module.exports = validateAssetLockStructureFactory;
