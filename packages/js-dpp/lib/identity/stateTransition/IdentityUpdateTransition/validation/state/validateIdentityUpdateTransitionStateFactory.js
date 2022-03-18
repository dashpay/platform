const ValidationResult = require('../../../../../validation/ValidationResult');
const IdentityNotFoundError = require('../../../../../errors/consensus/state/identity/IdentityNotFoundError');
const InvalidIdentityRevisionError = require('../../../../../errors/consensus/state/identity/InvalidIdentityRevisionError');
const IdentityPublicKey = require('../../../../IdentityPublicKey');
const IdentityPublicKeyIsReadOnlyError = require('../../../../../errors/consensus/state/identity/IdentityPublicKeyIsReadOnlyError');
const MissedSecurityLevelIdentityPublicKeyError = require('../../../../../errors/consensus/state/identity/MissedSecurityLevelIdentityPublicKeyError');

/**
 * @param {StateRepository} stateRepository
 * @return {validateIdentityUpdateTransitionState}
 */
function validateIdentityUpdateTransitionStateFactory(
  stateRepository,
) {
  /**
   * @typedef {validateIdentityUpdateTransitionState}
   * @param {IdentityUpdateTransition} stateTransition
   * @return {Promise<ValidationResult>}
   */
  // eslint-disable-next-line no-unused-vars
  async function validateIdentityUpdateTransitionState(stateTransition) {
    const result = new ValidationResult();

    const identityId = stateTransition.getIdentityId();
    const identity = await stateRepository.fetchIdentity(identityId);

    if (!identity) {
      result.addError(
        new IdentityNotFoundError(identityId.toBuffer()),
      );

      return result;
    }

    // Check revision
    if (identity.getRevision() !== stateTransition.getRevision() - 1) {
      result.addError(
        new InvalidIdentityRevisionError(identityId.toBuffer(), identity.getRevision()),
      );
    }

    const disablePublicKeys = stateTransition.getDisablePublicKeys();

    if (disablePublicKeys) {
      const identityPublicKeys = identity.getPublicKeys();

      disablePublicKeys.forEach((id) => {
        if (identityPublicKeys[id].getReadOnly()) {
          result.addError(
            new IdentityPublicKeyIsReadOnlyError(id),
          );
        }
      });

      if (!result.isValid()) {
        return result;
      }

      // Keys can only be disabled if another valid key is enabled in the same security level
      disablePublicKeys.forEach(
        (id) => identityPublicKeys[id].setDisabledAt(stateTransition.getPublicKeysDisabledAt()),
      );

      const addPublicKeys = stateTransition.addPublicKeys();
      if (addPublicKeys) {
        addPublicKeys.forEach((pk) => identityPublicKeys.push(pk));
      }

      const securityLevelsWithPublicKeys = {};

      identityPublicKeys.forEach((pk) => {
        const securityLevel = pk.getSecurityLevel();

        securityLevelsWithPublicKeys[securityLevel] = true;
      });

      const missedSecurityLevels = Object.values(IdentityPublicKey.SECURITY_LEVELS)
        .filter((level) => !Object.keys(securityLevelsWithPublicKeys).includes(level));

      if (missedSecurityLevels.length > 0) {
        missedSecurityLevels.forEach((securityLevel) => result.addError(
          new MissedSecurityLevelIdentityPublicKeyError(securityLevel),
        ));
      }
    }

    return result;
  }

  return validateIdentityUpdateTransitionState;
}

module.exports = validateIdentityUpdateTransitionStateFactory;
