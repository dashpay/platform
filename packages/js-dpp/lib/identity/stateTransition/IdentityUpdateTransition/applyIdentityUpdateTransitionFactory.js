/**
 * @param {StateRepository} stateRepository
 *
 * @returns {applyIdentityUpdateTransition}
 */
function applyIdentityUpdateTransitionFactory(
  stateRepository,
) {
  /**
   * Apply identity state transition
   *
   * @typedef applyIdentityUpdateTransition
   * @param {IdentityUpdateTransition} stateTransition
   * @returns {Promise<void>}
   */
  async function applyIdentityUpdateTransition(stateTransition) {
    const identityId = stateTransition.getIdentityId();

    const identity = await stateRepository.fetchIdentity(identityId);

    identity.setRevision(stateTransition.getRevision());

    if (stateTransition.getPublicKeyIdsToDisable()) {
      const identityPublicKeys = identity.getPublicKeys();

      stateTransition.getPublicKeyIdsToDisable()
        .forEach(
          (id) => identity.getPublicKeyById(id)
            .setDisabledAt(stateTransition.getPublicKeysDisabledAt().getTime()),
        );

      identity.setPublicKeys(identityPublicKeys);
    }

    if (stateTransition.getPublicKeysToAdd()) {
      const identityPublicKeys = identity
        .getPublicKeys()
        .concat(stateTransition.getPublicKeysToAdd());

      identity.setPublicKeys(identityPublicKeys);

      const publicKeyHashes = stateTransition
        .getPublicKeysToAdd()
        .map((publicKey) => publicKey.hash());

      await stateRepository.storeIdentityPublicKeyHashes(
        identity.getId(),
        publicKeyHashes,
      );
    }

    await stateRepository.storeIdentity(identity);
  }

  return applyIdentityUpdateTransition;
}

module.exports = applyIdentityUpdateTransitionFactory;
