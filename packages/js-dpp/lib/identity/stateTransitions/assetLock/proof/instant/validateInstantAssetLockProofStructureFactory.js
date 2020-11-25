const { InstantLock } = require('@dashevo/dashcore-lib');

const instantAssetLockProofSchema = require('../../../../../../schema/identity/stateTransition/assetLock/proof/instantAssetLockProof.json');

const convertBuffersToArrays = require('../../../../../util/convertBuffersToArrays');
const InvalidIdentityAssetLockProofError = require('../../../../../errors/InvalidIdentityAssetLockProofError');
const IdentityAssetLockProofMismatchError = require('../../../../../errors/IdentityAssetLockProofMismatchError');
const InvalidIdentityAssetLockProofSignatureError = require('../../../../../errors/InvalidIdentityAssetLockProofSignatureError');

/**
 * @param {JsonSchemaValidator} jsonSchemaValidator
 * @param {StateRepository} stateRepository
 * @param {boolean} skipAssetLockProofSignatureVerification
 * @returns {validateInstantAssetLockProofStructure}
 */
function validateInstantAssetLockProofStructureFactory(
  jsonSchemaValidator,
  stateRepository,
  skipAssetLockProofSignatureVerification,
) {
  /**
   * @typedef {validateInstantAssetLockProofStructure}
   * @param {RawInstantAssetLockProof} rawAssetLockProof
   * @param {Transaction} transaction
   */
  async function validateInstantAssetLockProofStructure(
    rawAssetLockProof,
    transaction,
  ) {
    const result = jsonSchemaValidator.validate(
      instantAssetLockProofSchema,
      convertBuffersToArrays(rawAssetLockProof),
    );

    if (!result.isValid()) {
      return result;
    }

    let instantLock;
    try {
      instantLock = InstantLock.fromBuffer(rawAssetLockProof.instantLock);
    } catch (e) {
      const error = new InvalidIdentityAssetLockProofError(e.message);

      result.addError(error);

      return result;
    }

    if (instantLock.txid !== transaction.id) {
      result.addError(new IdentityAssetLockProofMismatchError());

      return result;
    }

    if (!skipAssetLockProofSignatureVerification) {
      const smlStore = await stateRepository.fetchSMLStore();

      if (!await instantLock.verify(smlStore)) {
        result.addError(new InvalidIdentityAssetLockProofSignatureError());
      }
    }

    return result;
  }

  return validateInstantAssetLockProofStructure;
}

module.exports = validateInstantAssetLockProofStructureFactory;
