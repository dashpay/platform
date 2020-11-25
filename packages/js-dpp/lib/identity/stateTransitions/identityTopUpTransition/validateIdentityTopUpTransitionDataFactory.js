const ValidationResult = require('../../../validation/ValidationResult');

/**
 * @return {validateIdentityTopUpTransitionData}
 */
function validateIdentityTopUpTransitionDataFactory() {
  /**
   * @typedef validateIdentityTopUpTransitionData
   * @param {IdentityTopUpTransition} stateTransition
   * @return {ValidationResult}
   */
  // eslint-disable-next-line no-unused-vars
  async function validateIdentityTopUpTransitionData(stateTransition) {
    return new ValidationResult();
  }

  return validateIdentityTopUpTransitionData;
}

module.exports = validateIdentityTopUpTransitionDataFactory;
