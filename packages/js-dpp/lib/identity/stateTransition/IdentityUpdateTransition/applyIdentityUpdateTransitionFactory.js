/**
 * @param {StateRepository} stateRepository
 *
 * @returns {applyIdentityUpdateTransition}
 */
const IdentityPublicKey = require('../../IdentityPublicKey');

function applyIdentityUpdateTransitionFactory(
  stateRepository,
) {
  /**
   * Apply identity state transition
   *
   * @typedef {applyIdentityUpdateTransition}
   * @param {IdentityUpdateTransition} stateTransition
   * @returns {Promise<void>}
   */
  async function applyIdentityUpdateTransition(stateTransition) {
    const identityId = stateTransition.getIdentityId();
    const executionContext = stateTransition.getExecutionContext();

    await stateRepository.updateIdentityRevision(stateTransition.getRevision(), executionContext);

    if (stateTransition.getPublicKeyIdsToDisable()) {
      await stateRepository.disableIdentityKeys(
        identityId,
        stateTransition.getPublicKeyIdsToDisable(),
        stateTransition.getPublicKeysDisabledAt().getTime(),
        executionContext,
      );
    }

    if (stateTransition.getPublicKeysToAdd()) {
      const publicKeysToAdd = stateTransition.getPublicKeysToAdd()
        .map((publicKey) => {
          const rawPublicKey = publicKey.toObject({ skipSignature: true });

          return new IdentityPublicKey(rawPublicKey);
        });

      await stateRepository.addKeysToIdentity(
        identityId,
        publicKeysToAdd,
        executionContext,
      );
    }
  }

  return applyIdentityUpdateTransition;
}

module.exports = applyIdentityUpdateTransitionFactory;
