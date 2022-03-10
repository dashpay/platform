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
    const outPoint = stateTransition.getAssetLockProof().getOutPoint();

    const identityId = stateTransition.getIdentityId();

    const identity = await stateRepository.fetchIdentity(identityId);

    identity.setRevision(stateTransition.getRevision());

    if (stateTransition.getDisablePublicKeys()) {
      const identityPublicKeys = identity.getPublicKeys();

      stateTransition.getDisablePublicKeys()
        .forEach(
          (id) => identityPublicKeys[id].setDisabledAt(stateTransition.getPublicKeysDisabledAt()),
        );

      identity.setPublicKeys(identityPublicKeys);
    }

    if (stateTransition.getAddPublicKeys()) {
      const identityPublicKeys = identity
        .getPublicKeys()
        .concat(stateTransition.getAddPublicKeys());

      identity.setPublicKeys(identityPublicKeys);
    }

    await stateRepository.storeIdentity(identity);

    await stateRepository.markAssetLockTransactionOutPointAsUsed(outPoint);
  }

  return applyIdentityUpdateTransition;
}

module.exports = applyIdentityUpdateTransitionFactory;
