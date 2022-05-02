/**
 * Apply data contract state transition (factory)
 *
 * @param {StateRepository} stateRepository
 *
 * @returns {applyDataContractCreateTransition}
 */
function applyDataContractCreateTransitionFactory(stateRepository) {
  /**
   * Apply data contract state transition
   *
   * @typedef applyDataContractCreateTransition
   *
   * @param {DataContractCreateTransition} stateTransition
   *
   * @return {Promise<void>}
   */
  async function applyDataContractCreateTransition(stateTransition) {
    const executionContext = stateTransition.getExecutionContext();

    await stateRepository.storeDataContract(
      stateTransition.getDataContract(),
      executionContext,
    );
  }

  return applyDataContractCreateTransition;
}

module.exports = applyDataContractCreateTransitionFactory;
