const ValidationResult = require('../../../../../validation/ValidationResult');
const InvalidIdentityRevisionError = require('../../../../../errors/consensus/state/identity/InvalidIdentityRevisionError');
const IdentityPublicKey = require('../../../../IdentityPublicKey');
const IdentityPublicKeyIsReadOnlyError = require('../../../../../errors/consensus/state/identity/IdentityPublicKeyIsReadOnlyError');
const InvalidIdentityPublicKeyIdError = require('../../../../../errors/consensus/state/identity/InvalidIdentityPublicKeyIdError');
const MissedSecurityLevelIdentityPublicKeyError = require('../../../../../errors/consensus/state/identity/MissedSecurityLevelIdentityPublicKeyError');
const Identity = require('../../../../Identity');
const IdentityPublicKeyDisabledAtWindowViolationError = require('../../../../../errors/consensus/state/identity/IdentityPublicKeyDisabledAtWindowViolationError');

// security levels which must have at least one key
const SECURITY_LEVELS = [
  IdentityPublicKey.SECURITY_LEVELS.MASTER,
];

const BLOCK_TIME_WINDOW_MINUTES = 5;

/**
 * @param {StateRepository} stateRepository
 * @param {validatePublicKeys} validatePublicKeys
 * @param {validatePublicKeysAreEnabled} validatePublicKeysAreEnabled
 * @return {validateIdentityUpdateTransitionState}
 */
function validateIdentityUpdateTransitionStateFactory(
  stateRepository,
  validatePublicKeys,
  validatePublicKeysAreEnabled,
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
    const storedIdentity = await stateRepository.fetchIdentity(identityId);

    // copy identity
    const identity = (new Identity(storedIdentity.toObject()));

    // Check revision
    if (identity.getRevision() !== stateTransition.getRevision() - 1) {
      result.addError(
        new InvalidIdentityRevisionError(identityId.toBuffer(), identity.getRevision()),
      );
    }

    const disablePublicKeys = stateTransition.getPublicKeyIdsToDisable();

    if (disablePublicKeys) {
      disablePublicKeys.forEach((id) => {
        if (!identity.getPublicKeyById(id)) {
          result.addError(
            new InvalidIdentityPublicKeyIdError(id),
          );
        } else if (identity.getPublicKeyById(id).getReadOnly()) {
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

      // Calculate time window for timestamps
      const {
        time: {
          seconds: lastBlockHeaderTimeSeconds,
        },
      } = await stateRepository.fetchLatestPlatformBlockHeader();

      // Get last block header time in milliseconds
      const lastBlockHeaderTime = lastBlockHeaderTimeSeconds * 1000;

      // Define time window
      const timeWindowStart = new Date(lastBlockHeaderTime);
      timeWindowStart.setMinutes(
        timeWindowStart.getMinutes() - BLOCK_TIME_WINDOW_MINUTES,
      );

      const timeWindowEnd = new Date(lastBlockHeaderTime);
      timeWindowEnd.setMinutes(
        timeWindowEnd.getMinutes() + BLOCK_TIME_WINDOW_MINUTES,
      );

      const disabledAtTime = stateTransition.getPublicKeysDisabledAt();

      if (disabledAtTime < timeWindowStart || disabledAtTime > timeWindowEnd) {
        result.addError(
          new IdentityPublicKeyDisabledAtWindowViolationError(
            disabledAtTime,
            timeWindowStart,
            timeWindowEnd,
          ),
        );
      }

      if (!result.isValid()) {
        return result;
      }
    }

    const addPublicKeys = stateTransition.getPublicKeysToAdd();
    if (addPublicKeys) {
      result.merge(
        validatePublicKeysAreEnabled(addPublicKeys.map((pk) => pk.toObject())),
      );

      if (!result.isValid()) {
        return result;
      }

      addPublicKeys.forEach((pk) => identity.getPublicKeys().push(pk));
    }

    result.merge(
      validatePublicKeys(identity.getPublicKeys().map((pk) => pk.toObject())),
    );

    return result;
  }

  return validateIdentityUpdateTransitionState;
}

module.exports = validateIdentityUpdateTransitionStateFactory;
