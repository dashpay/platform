const ValidationResult = require('../../../../../validation/ValidationResult');
const IdentityNotFoundError = require('../../../../../errors/consensus/state/identity/IdentityNotFoundError');
const InvalidIdentityRevisionError = require('../../../../../errors/consensus/state/identity/InvalidIdentityRevisionError');
const IdentityPublicKey = require('../../../../IdentityPublicKey');
const IdentityPublicKeyIsReadOnlyError = require('../../../../../errors/consensus/state/identity/IdentityPublicKeyIsReadOnlyError');
const MissedSecurityLevelIdentityPublicKeyError = require('../../../../../errors/consensus/state/identity/MissedSecurityLevelIdentityPublicKeyError');

// security levels which must have at least one key
const SECURITY_LEVELS = [
  IdentityPublicKey.SECURITY_LEVELS.MASTER,
];

/**
 * @param {StateRepository} stateRepository
 * @param {validatePublicKeys} validatePublicKeys
 * @return {validateIdentityUpdateTransitionState}
 */
function validateIdentityUpdateTransitionStateFactory(
  stateRepository,
  validatePublicKeys,
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

    const addPublicKeys = stateTransition.getAddPublicKeys();
    if (addPublicKeys) {
      addPublicKeys.forEach((pk) => identity.getPublicKeys().push(pk));
    }

    const disablePublicKeys = stateTransition.getDisablePublicKeys();

    if (disablePublicKeys) {
      disablePublicKeys.forEach((id) => {
        if (identity.getPublicKeyById(id).getReadOnly()) {
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
        (id) => identity.getPublicKeyById(id)
          .setDisabledAt(stateTransition.getPublicKeysDisabledAt()),
      );

      const securityLevelsWithKeys = new Set();

      SECURITY_LEVELS.forEach((securityLevel) => {
        identity.getPublicKeys()
          .filter((pk) => pk.getSecurityLevel() === securityLevel)
          .forEach((pk) => {
            if (!pk.getDisabledAt()) {
              securityLevelsWithKeys.add(securityLevel);
            }
          });
      });

      SECURITY_LEVELS.forEach((securityLevel) => {
        if (!securityLevelsWithKeys.has(securityLevel)) {
          result.addError(new MissedSecurityLevelIdentityPublicKeyError(securityLevel));
        }
      });

      if (!result.isValid()) {
        return result;
      }
    }

    result.merge(
      validatePublicKeys(identity.getPublicKeys().map((pk) => pk.toObject())),
    );

    return result;
  }

  return validateIdentityUpdateTransitionState;
}

module.exports = validateIdentityUpdateTransitionStateFactory;
