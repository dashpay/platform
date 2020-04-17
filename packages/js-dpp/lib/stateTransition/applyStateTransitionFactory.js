const stateTransitionTypes = require('./stateTransitionTypes');

/**
 * Update state by applying transition (factory)
 *
 * @param {applyDataContractCreateTransition} applyDataContractCreateTransition
 * @param {applyDocumentsBatchTransition} applyDocumentsBatchTransition
 * @param {applyIdentityCreateTransition} applyIdentityCreateTransition
 *
 * @returns {applyStateTransition}
 */
function applyStateTransitionFactory(
  applyDataContractCreateTransition,
  applyDocumentsBatchTransition,
  applyIdentityCreateTransition,
) {
  /* map apply functions */
  const typesToFunction = {
    [stateTransitionTypes.DATA_CONTRACT_CREATE]: applyDataContractCreateTransition,
    [stateTransitionTypes.DOCUMENTS_BATCH]: applyDocumentsBatchTransition,
    [stateTransitionTypes.IDENTITY_CREATE]: applyIdentityCreateTransition,
  };

  /**
   * Update state by applying transition
   *
   * @typedef applyStateTransition
   *
   * @param {AbstractStateTransition} stateTransition
   *
   * @returns {Promise<void>}
   */
  async function applyStateTransition(stateTransition) {
    await typesToFunction[stateTransition.getType()](stateTransition);
  }

  return applyStateTransition;
}

module.exports = applyStateTransitionFactory;
