const ValidationResult = require('../../../../../validation/ValidationResult');

const IdentityAlreadyExistsError = require('../../../../../errors/consensus/state/identity/IdentityAlreadyExistsError');

/**
 * @param {StateRepository} stateRepository
 * @param {validateIdentityPublicKeysUniqueness} validateIdentityPublicKeysUniqueness
 * @return {validateIdentityCreateTransitionState}
 */
function validateIdentityCreateTransitionStateFactory(
  stateRepository,
  validateIdentityPublicKeysUniqueness,
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
    const identityId = stateTransition.getIdentityId();
    const identity = await stateRepository.fetchIdentity(identityId);

    if (identity) {
      result.addError(
        new IdentityAlreadyExistsError(identityId.toBuffer()),
      );
    }

    if (!result.isValid()) {
      return result;
    }

    result.merge(
      await validateIdentityPublicKeysUniqueness(
        stateTransition.getPublicKeys(),
      ),
    );

    return result;
  }

  return validateIdentityCreateTransitionState;
}

module.exports = validateIdentityCreateTransitionStateFactory;
