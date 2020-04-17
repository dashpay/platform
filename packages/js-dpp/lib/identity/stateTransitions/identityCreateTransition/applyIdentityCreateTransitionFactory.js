const Identity = require('../../Identity');

const { convertSatoshiToCredits } = require('../../creditsConverter');

/**
 * @param {StateRepository} stateRepository
 * @param {getLockedTransactionOutput} getLockedTransactionOutput
 *
 * @returns {applyIdentityCreateTransition}
 */
function applyIdentityCreateTransitionFactory(
  stateRepository,
  getLockedTransactionOutput,
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
    const output = await getLockedTransactionOutput(stateTransition.getLockedOutPoint());
    const creditsAmount = convertSatoshiToCredits(output.satoshis);

    const identity = new Identity({
      id: stateTransition.getIdentityId(),
      publicKeys: stateTransition.getPublicKeys().map((key) => key.toJSON()),
      balance: creditsAmount,
    });

    await stateRepository.storeIdentity(identity);
  }

  return applyIdentityCreateTransition;
}

module.exports = applyIdentityCreateTransitionFactory;
