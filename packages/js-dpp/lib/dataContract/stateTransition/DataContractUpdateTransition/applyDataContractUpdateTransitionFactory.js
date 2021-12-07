/**
 * Apply data contract state transition (factory)
 *
 * @param {StateRepository} stateRepository
 *
 * @returns {applyDataContractUpdateTransition}
 */
function applyDataContractUpdateTransitionFactory(stateRepository) {
  /**
   * Apply data contract state transition
   *
   * @typedef applyDataContractUpdateTransition
   *
   * @param {DataContractCreateTransition} stateTransition
   *
   * @return {Promise<void>}
   */
  async function applyDataContractUpdateTransition(stateTransition) {
    await stateRepository.storeDataContract(
      stateTransition.getDataContract(),
    );
  }

  return applyDataContractUpdateTransition;
}

module.exports = applyDataContractUpdateTransitionFactory;
