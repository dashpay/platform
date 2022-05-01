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
    const executionContext = stateTransition.getExecutionContext();

    await stateRepository.storeDataContract(
      stateTransition.getDataContract(),
      executionContext,
    );
  }

  return applyDataContractUpdateTransition;
}

module.exports = applyDataContractUpdateTransitionFactory;
