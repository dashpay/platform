const ValidationResult = require('../../../../../validation/ValidationResult');

/**
 * @return {validateIdentityTopUpTransitionState}
 */
function validateIdentityTopUpTransitionStateFactory() {
  /**
   * @typedef {validateIdentityTopUpTransitionState}
   * @param {IdentityTopUpTransition} stateTransition
   * @return {Promise<ValidationResult>}
   */
  // eslint-disable-next-line no-unused-vars
  async function validateIdentityTopUpTransitionState(stateTransition) {
    return new ValidationResult();
  }

  return validateIdentityTopUpTransitionState;
}

module.exports = validateIdentityTopUpTransitionStateFactory;
