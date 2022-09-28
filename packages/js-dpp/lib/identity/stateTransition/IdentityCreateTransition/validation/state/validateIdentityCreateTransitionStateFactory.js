const ValidationResult = require('../../../../../validation/ValidationResult');

const IdentityAlreadyExistsError = require('../../../../../errors/consensus/state/identity/IdentityAlreadyExistsError');

/**
 * @param {StateRepository} stateRepository
 * @return {validateIdentityCreateTransitionState}
 */
function validateIdentityCreateTransitionStateFactory(
  stateRepository,
) {
  /**
   *
   * Do we need to check that key ids are incremental?
   *
   * For later versions:
   * 1. We need to check that outpoint exists (not now)
   * 2. Verify ownership proof signature, as it requires special transaction to be implemented
   */

  /**
   * @typedef {validateIdentityCreateTransitionState}
   * @param {IdentityCreateTransition} stateTransition
   * @return {ValidationResult}
   */
  async function validateIdentityCreateTransitionState(stateTransition) {
    const result = new ValidationResult();

    // Check if identity with such id already exists
    const executionContext = stateTransition.getExecutionContext();
    const identityId = stateTransition.getIdentityId();
    const identity = await stateRepository.fetchIdentity(identityId, executionContext);

    if (executionContext.isDryRun()) {
      return result;
    }

    if (identity) {
      result.addError(
        new IdentityAlreadyExistsError(identityId.toBuffer()),
      );
    }

    return result;
  }

  return validateIdentityCreateTransitionState;
}

module.exports = validateIdentityCreateTransitionStateFactory;
