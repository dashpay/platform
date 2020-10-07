const identityCreateTransitionSchema = require('../../../../schema/identity/stateTransition/identityCreate');

/**
 * @param {JsonSchemaValidator} jsonSchemaValidator
 * @param {validatePublicKeys} validatePublicKeys
 * @return {validateIdentityCreateTransitionStructure}
 */
function validateIdentityCreateTransitionStructureFactory(
  jsonSchemaValidator,
  validatePublicKeys,
) {
  /**
   * @typedef validateIdentityCreateTransitionStructure
   * @param {RawIdentityCreateTransition} rawStateTransition
   * @return {ValidationResult}
   */
  function validateIdentityCreateTransitionStructure(rawStateTransition) {
    // Validate state transition against JSON Schema
    const result = jsonSchemaValidator.validate(
      identityCreateTransitionSchema,
      rawStateTransition,
    );

    if (!result.isValid()) {
      return result;
    }

    result.merge(
      validatePublicKeys(rawStateTransition.publicKeys),
    );

    return result;
  }

  return validateIdentityCreateTransitionStructure;
}

module.exports = validateIdentityCreateTransitionStructureFactory;
