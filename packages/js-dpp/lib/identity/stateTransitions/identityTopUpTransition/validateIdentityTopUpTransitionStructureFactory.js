const IdentityTopUpTransition = require('./IdentityTopUpTransition');

const encodeObjectProperties = require('../../../util/encoding/encodeObjectProperties');

const identityTopUpTransitionSchema = require('../../../../schema/identity/stateTransition/identityTopUp.json');

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
    // Validate state transition against JSON Schema
    const jsonStateTransition = encodeObjectProperties(
      rawStateTransition,
      IdentityTopUpTransition.ENCODED_PROPERTIES,
    );

    return jsonSchemaValidator.validate(
      identityTopUpTransitionSchema,
      jsonStateTransition,
    );
  }

  return validateIdentityTopUpTransitionStructure;
}

module.exports = validateIdentityTopUpTransitionStructureFactory;
