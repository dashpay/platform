const identityTopUpTransitionSchema = require('../../../../../../schema/identity/stateTransition/identityTopUp.json');

const convertBuffersToArrays = require('../../../../../util/convertBuffersToArrays');
const Identifier = require('../../../../../identifier/Identifier');

/**
 * @param {JsonSchemaValidator} jsonSchemaValidator
 * @param {validateIdentityExistence} validateIdentityExistence
 * @param {Object.<number, Function>} proofValidationFunctionsByType
 * @return {validateIdentityTopUpTransitionBasic}
 */
function validateIdentityTopUpTransitionBasicFactory(
  jsonSchemaValidator,
  validateIdentityExistence,
  proofValidationFunctionsByType,
) {
  /**
   * @typedef {validateIdentityTopUpTransitionBasic}
   * @param {RawIdentityTopUpTransition} rawStateTransition
   * @return {ValidationResult}
   */
  async function validateIdentityTopUpTransitionBasic(rawStateTransition) {
    const result = jsonSchemaValidator.validate(
      identityTopUpTransitionSchema,
      convertBuffersToArrays(rawStateTransition),
    );

    if (!result.isValid()) {
      return result;
    }

    result.merge(
      await validateIdentityExistence(
        new Identifier(rawStateTransition.identityId),
      ),
    );

    if (!result.isValid()) {
      return result;
    }

    const proofValidationFunction = proofValidationFunctionsByType[
      rawStateTransition.assetLockProof.type
    ];

    const assetLockProofValidationResult = await proofValidationFunction(
      rawStateTransition.assetLockProof,
    );

    result.merge(
      assetLockProofValidationResult,
    );

    return result;
  }

  return validateIdentityTopUpTransitionBasic;
}

module.exports = validateIdentityTopUpTransitionBasicFactory;
