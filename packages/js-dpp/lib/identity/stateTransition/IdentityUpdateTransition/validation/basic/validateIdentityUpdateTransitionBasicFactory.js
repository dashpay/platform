const identityUpdateTransitionSchema = require('../../../../../../schema/identity/stateTransition/identityUpdate.json');
const convertBuffersToArrays = require('../../../../../util/convertBuffersToArrays');

/**
 * @param {JsonSchemaValidator} jsonSchemaValidator
 * @param {validateProtocolVersion} validateProtocolVersion
 * @param {validatePublicKeys} validatePublicKeys
 * @param {validatePublicKeySignatures} validatePublicKeySignatures
 *
 * @return {validateIdentityUpdateTransitionBasic}
 */
function validateIdentityUpdateTransitionBasicFactory(
  jsonSchemaValidator,
  validateProtocolVersion,
  validatePublicKeys,
  validatePublicKeySignatures,
) {
  /**
   * @typedef validateIdentityUpdateTransitionBasic
   * @param {RawIdentityUpdateTransition} rawStateTransition
   * @param {StateTransitionExecutionContext} executionContext
   * @return {Promise<ValidationResult>}
   */
  // eslint-disable-next-line no-unused-vars
  async function validateIdentityUpdateTransitionBasic(rawStateTransition, executionContext) {
    const result = jsonSchemaValidator.validate(
      identityUpdateTransitionSchema,
      convertBuffersToArrays(rawStateTransition),
    );

    if (!result.isValid()) {
      return result;
    }

    result.merge(
      validateProtocolVersion(rawStateTransition.protocolVersion),
    );

    if (!result.isValid()) {
      return result;
    }

    if (rawStateTransition.addPublicKeys) {
      result.merge(
        validatePublicKeys(rawStateTransition.addPublicKeys),
      );

      if (!result.isValid()) {
        return result;
      }

      result.merge(
        await validatePublicKeySignatures(
          rawStateTransition,
          rawStateTransition.addPublicKeys,
          executionContext,
        ),
      );
    }

    return result;
  }

  return validateIdentityUpdateTransitionBasic;
}

module.exports = validateIdentityUpdateTransitionBasicFactory;
