const DashCoreLib = require('@dashevo/dashcore-lib');

const instantAssetLockProofSchema = require('../../../../../schema/identity/stateTransition/assetLockProof/instantAssetLockProof.json');

const convertBuffersToArrays = require('../../../../util/convertBuffersToArrays');
const InvalidInstantAssetLockProofError = require('../../../../errors/consensus/basic/identity/InvalidInstantAssetLockProofError');
const IdentityAssetLockProofLockedTransactionMismatchError = require('../../../../errors/consensus/basic/identity/IdentityAssetLockProofLockedTransactionMismatchError');
const InvalidIdentityAssetLockProofSignatureError = require('../../../../errors/consensus/basic/identity/InvalidInstantAssetLockProofSignatureError');

/**
 * @param {JsonSchemaValidator} jsonSchemaValidator
 * @param {StateRepository} stateRepository
 * @param {validateAssetLockTransaction} validateAssetLockTransaction
 * @returns {validateInstantAssetLockProofStructure}
 */
function validateInstantAssetLockProofStructureFactory(
  jsonSchemaValidator,
  stateRepository,
  validateAssetLockTransaction,
) {
  /**
   * @typedef {validateInstantAssetLockProofStructure}
   * @param {RawInstantAssetLockProof} rawAssetLockProof
   */
  async function validateInstantAssetLockProofStructure(
    rawAssetLockProof,
  ) {
    const result = jsonSchemaValidator.validate(
      instantAssetLockProofSchema,
      convertBuffersToArrays(rawAssetLockProof),
    );

    if (!result.isValid()) {
      return result;
    }

    const { InstantLock } = DashCoreLib;

    let instantLock;
    try {
      instantLock = InstantLock.fromBuffer(rawAssetLockProof.instantLock);
    } catch (e) {
      const error = new InvalidInstantAssetLockProofError(e.message);

      error.setValidationError(e);

      result.addError(error);

      return result;
    }

    if (!await stateRepository.verifyInstantLock(instantLock)) {
      result.addError(new InvalidIdentityAssetLockProofSignatureError());

      return result;
    }

    const validateAssetLockTransactionResult = await validateAssetLockTransaction(
      rawAssetLockProof.transaction,
      rawAssetLockProof.outputIndex,
    );

    result.merge(validateAssetLockTransactionResult);

    if (!result.isValid()) {
      return result;
    }

    /**
     * @typedef {Transaction} transaction
     * @typedef {Buffer} publicKeyHash
     */
    const { publicKeyHash, transaction } = validateAssetLockTransactionResult.getData();

    if (instantLock.txid !== transaction.id) {
      result.addError(
        new IdentityAssetLockProofLockedTransactionMismatchError(
          Buffer.from(instantLock.txid, 'hex'),
          Buffer.from(transaction.id, 'hex'),
        ),
      );

      return result;
    }

    result.setData(publicKeyHash);

    return result;
  }

  return validateInstantAssetLockProofStructure;
}

module.exports = validateInstantAssetLockProofStructureFactory;
