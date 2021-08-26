const { Transaction } = require('@dashevo/dashcore-lib');
const chainAssetLockProofSchema = require('../../../../../schema/identity/stateTransition/assetLockProof/chainAssetLockProof.json');

const convertBuffersToArrays = require('../../../../util/convertBuffersToArrays');
const InvalidAssetLockProofCoreChainHeightError = require('../../../../errors/consensus/basic/identity/InvalidAssetLockProofCoreChainHeightError');
const IdentityAssetLockTransactionIsNotFoundError = require('../../../../errors/consensus/basic/identity/IdentityAssetLockTransactionIsNotFoundError');
const InvalidAssetLockProofTransactionHeightError = require('../../../../errors/consensus/basic/identity/InvalidAssetLockProofTransactionHeightError');

/**
 * @param {JsonSchemaValidator} jsonSchemaValidator
 * @param {StateRepository} stateRepository
 * @param {validateAssetLockTransaction} validateAssetLockTransaction
 * @returns {validateChainAssetLockProofStructure}
 */
function validateChainAssetLockProofStructureFactory(
  jsonSchemaValidator,
  stateRepository,
  validateAssetLockTransaction,
) {
  /**
   * @typedef {validateChainAssetLockProofStructure}
   * @param {RawChainAssetLockProof} rawAssetLockProof
   * @returns {ValidationResult}
   */
  async function validateChainAssetLockProofStructure(
    rawAssetLockProof,
  ) {
    const result = jsonSchemaValidator.validate(
      chainAssetLockProofSchema,
      convertBuffersToArrays(rawAssetLockProof),
    );

    if (!result.isValid()) {
      return result;
    }

    const {
      coreChainLockedHeight: proofCoreChainLockedHeight,
      outPoint: outPointBuffer,
    } = rawAssetLockProof;

    const latestPlatformBlockHeader = await stateRepository.fetchLatestPlatformBlockHeader();

    const { coreChainLockedHeight: currentCoreChainLockedHeight } = latestPlatformBlockHeader;

    if (currentCoreChainLockedHeight < proofCoreChainLockedHeight) {
      result.addError(
        new InvalidAssetLockProofCoreChainHeightError(
          proofCoreChainLockedHeight,
          currentCoreChainLockedHeight,
        ),
      );

      return result;
    }

    const outPoint = Transaction.parseOutPointBuffer(outPointBuffer);
    const { outputIndex, transactionHash } = outPoint;

    const rawTransaction = await stateRepository.fetchTransaction(transactionHash);

    if (rawTransaction === null) {
      result.addError(new IdentityAssetLockTransactionIsNotFoundError(outPointBuffer));

      return result;
    }

    if (!rawTransaction.height || proofCoreChainLockedHeight < rawTransaction.height) {
      result.addError(
        new InvalidAssetLockProofTransactionHeightError(
          proofCoreChainLockedHeight,
          rawTransaction.height,
        ),
      );

      return result;
    }

    const validateAssetLockTransactionResult = await validateAssetLockTransaction(
      rawTransaction.data,
      outputIndex,
    );

    result.merge(validateAssetLockTransactionResult);

    if (!result.isValid()) {
      return result;
    }

    const { publicKeyHash } = validateAssetLockTransactionResult.getData();

    result.setData(publicKeyHash);

    return result;
  }

  return validateChainAssetLockProofStructure;
}

module.exports = validateChainAssetLockProofStructureFactory;
