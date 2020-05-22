const Identity = require('../../Identity');

const { convertSatoshiToCredits } = require('../../creditsConverter');

/**
 * @param {StateRepository} stateRepository
 * @param {fetchConfirmedLockTransactionOutput} fetchConfirmedLockTransactionOutput
 *
 * @returns {applyIdentityCreateTransition}
 */
function applyIdentityCreateTransitionFactory(
  stateRepository,
  fetchConfirmedLockTransactionOutput,
) {
  /**
   * Apply identity state transition
   *
   * @typedef applyIdentityCreateTransition
   *
   * @param {IdentityCreateTransition} stateTransition
   *
   * @return {Promise<void>}
   */
  async function applyIdentityCreateTransition(stateTransition) {
    const output = await fetchConfirmedLockTransactionOutput(stateTransition.getLockedOutPoint());
    const creditsAmount = convertSatoshiToCredits(output.satoshis);

    const identity = new Identity({
      id: stateTransition.getIdentityId(),
      publicKeys: stateTransition.getPublicKeys().map((key) => key.toJSON()),
      balance: creditsAmount,
    });

    await stateRepository.storeIdentity(identity);

    const [firstPublicKey] = identity.getPublicKeys();
    await stateRepository.storePublicKeyIdentityId(
      firstPublicKey.hash(),
      identity.getId(),
    );
  }

  return applyIdentityCreateTransition;
}

module.exports = applyIdentityCreateTransitionFactory;
