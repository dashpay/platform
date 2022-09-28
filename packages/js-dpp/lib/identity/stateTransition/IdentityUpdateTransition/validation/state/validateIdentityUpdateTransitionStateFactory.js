const ValidationResult = require('../../../../../validation/ValidationResult');
const InvalidIdentityRevisionError = require('../../../../../errors/consensus/state/identity/InvalidIdentityRevisionError');
const IdentityPublicKeyIsReadOnlyError = require('../../../../../errors/consensus/state/identity/IdentityPublicKeyIsReadOnlyError');
const InvalidIdentityPublicKeyIdError = require('../../../../../errors/consensus/state/identity/InvalidIdentityPublicKeyIdError');
const Identity = require('../../../../Identity');
const IdentityPublicKeyDisabledAtWindowViolationError = require('../../../../../errors/consensus/state/identity/IdentityPublicKeyDisabledAtWindowViolationError');
const validateTimeInBlockTimeWindow = require('../../../../../blockTimeWindow/validateTimeInBlockTimeWindow');
const IdentityPublicKeyIsDisabledError = require('../../../../../errors/consensus/state/identity/IdentityPublicKeyIsDisabledError');

/**
 * @param {StateRepository} stateRepository
 * @param {validatePublicKeys} validatePublicKeys
 * @param {validateRequiredPurposeAndSecurityLevel} validateRequiredPurposeAndSecurityLevel
 * @return {validateIdentityUpdateTransitionState}
 */
function validateIdentityUpdateTransitionStateFactory(
  stateRepository,
  validatePublicKeys,
  validateRequiredPurposeAndSecurityLevel,
) {
  /**
   * @typedef {validateIdentityUpdateTransitionState}
   * @param {IdentityUpdateTransition} stateTransition
   * @return {Promise<ValidationResult>}
   */
  // eslint-disable-next-line no-unused-vars
  async function validateIdentityUpdateTransitionState(stateTransition) {
    const result = new ValidationResult();

    const executionContext = stateTransition.getExecutionContext();
    const identityId = stateTransition.getIdentityId();

    const storedIdentity = await stateRepository.fetchIdentity(identityId, executionContext);

    if (executionContext.isDryRun()) {
      return result;
    }

    // copy identity
    const identity = new Identity(storedIdentity.toObject());

    // Check revision
    if (identity.getRevision() !== stateTransition.getRevision() - 1) {
      result.addError(
        new InvalidIdentityRevisionError(identityId.toBuffer(), identity.getRevision()),
      );

      return result;
    }

    const publicKeyIdsToDisable = stateTransition.getPublicKeyIdsToDisable();

    if (publicKeyIdsToDisable) {
      publicKeyIdsToDisable.forEach((id) => {
        const publicKeyToDisable = identity.getPublicKeyById(id);

        if (!publicKeyToDisable) {
          result.addError(
            new InvalidIdentityPublicKeyIdError(id),
          );

          return;
        }

        if (publicKeyToDisable.isReadOnly()) {
          result.addError(
            new IdentityPublicKeyIsReadOnlyError(id),
          );

          return;
        }

        if (publicKeyToDisable.isDisabled()) {
          result.addError(
            new IdentityPublicKeyIsDisabledError(id),
          );
        }
      });

      if (!result.isValid()) {
        return result;
      }

      // Keys can only be disabled if another valid key is enabled in the same security level
      publicKeyIdsToDisable.forEach(
        (id) => identity.getPublicKeyById(id)
          .setDisabledAt(stateTransition.getPublicKeysDisabledAt().getTime()),
      );

      // Calculate time window for timestamps
      const {
        time: {
          seconds: lastBlockHeaderTimeSeconds,
        },
      } = await stateRepository.fetchLatestPlatformBlockHeader();

      // Get last block header time in milliseconds
      const lastBlockHeaderTime = lastBlockHeaderTimeSeconds * 1000;

      const disabledAtTime = stateTransition.getPublicKeysDisabledAt();

      const validateTimeWindowResult = validateTimeInBlockTimeWindow(
        lastBlockHeaderTime,
        disabledAtTime.getTime(),
      );

      if (!validateTimeWindowResult.isValid()) {
        result.addError(
          new IdentityPublicKeyDisabledAtWindowViolationError(
            disabledAtTime,
            validateTimeWindowResult.getTimeWindowStart(),
            validateTimeWindowResult.getTimeWindowEnd(),
          ),
        );

        return result;
      }
    }

    const publicKeysToAdd = stateTransition.getPublicKeysToAdd();
    if (publicKeysToAdd) {
      const identityPublicKeys = identity.getPublicKeys();

      publicKeysToAdd.forEach((pk) => identityPublicKeys.push(pk));

      identity.setPublicKeys(identityPublicKeys);

      // validate new fields with existing once to make sure that keys are unique and so on
      result.merge(
        validatePublicKeys(
          identity.getPublicKeys().map((pk) => pk.toObject()),
        ),
      );

      if (!result.isValid()) {
        return result;
      }
    }

    const rawPublicKeys = identity.getPublicKeys().map((pk) => pk.toObject());

    result.merge(
      validateRequiredPurposeAndSecurityLevel(rawPublicKeys),
    );

    return result;
  }

  return validateIdentityUpdateTransitionState;
}

module.exports = validateIdentityUpdateTransitionStateFactory;
