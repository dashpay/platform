const identityTopUpTransitionSchema = require('../../../../schema/identity/stateTransition/identityTopUp.json');

const convertBuffersToArrays = require('../../../util/convertBuffersToArrays');

/**
 * @param {JsonSchemaValidator} jsonSchemaValidator
 * @return {validateIdentityTopUpTransitionStructure}
 */
function validateIdentityTopUpTransitionStructureFactory(jsonSchemaValidator) {
  /**
   * @typedef {validateIdentityTopUpTransitionStructure}
   * @param {RawIdentityTopUpTransition} rawStateTransition
   * @return {ValidationResult}
   */
  function validateIdentityTopUpTransitionStructure(rawStateTransition) {
    return jsonSchemaValidator.validate(
      identityTopUpTransitionSchema,
      convertBuffersToArrays(rawStateTransition),
    );
  }

  return validateIdentityTopUpTransitionStructure;
}

module.exports = validateIdentityTopUpTransitionStructureFactory;
