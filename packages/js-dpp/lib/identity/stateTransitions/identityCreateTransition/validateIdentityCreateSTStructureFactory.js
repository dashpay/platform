const ValidationResult = require('../../../validation/ValidationResult');
const IdentityStateTransition = require('./IdentityCreateTransition');

/**
 * @param {validatePublicKeys} validatePublicKeys
 * @return {validateIdentityCreateST}
 */
function validateIdentityCreateSTStructureFactory(
  validatePublicKeys,
) {
  /**
   * @typedef validateIdentityCreateST
   * @param {IdentityCreateTransition} identityStateTransition
   * @return {ValidationResult}
   */
  function validateIdentityCreateST(identityStateTransition) {
    let rawIdentityStateTransition;

    if (identityStateTransition instanceof IdentityStateTransition) {
      rawIdentityStateTransition = identityStateTransition.toJSON();
    } else {
      rawIdentityStateTransition = identityStateTransition;
    }

    const result = new ValidationResult();

    result.merge(
      validatePublicKeys(rawIdentityStateTransition.publicKeys),
    );

    return result;
  }

  return validateIdentityCreateST;
}

module.exports = validateIdentityCreateSTStructureFactory;
