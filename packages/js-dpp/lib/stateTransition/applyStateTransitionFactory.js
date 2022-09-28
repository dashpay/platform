const stateTransitionTypes = require('./stateTransitionTypes');

/**
 * Update state by applying transition (factory)
 *
 * @param {applyDataContractCreateTransition} applyDataContractCreateTransition
 * @param {applyDataContractUpdateTransition} applyDataContractUpdateTransition
 * @param {applyDocumentsBatchTransition} applyDocumentsBatchTransition
 * @param {applyIdentityCreateTransition} applyIdentityCreateTransition
 * @param {applyIdentityTopUpTransition} applyIdentityTopUpTransition
 * @param {applyIdentityUpdateTransition} applyIdentityUpdateTransition
 *
 * @returns {applyStateTransition}
 */
function applyStateTransitionFactory(
  applyDataContractCreateTransition,
  applyDataContractUpdateTransition,
  applyDocumentsBatchTransition,
  applyIdentityCreateTransition,
  applyIdentityTopUpTransition,
  applyIdentityUpdateTransition,
) {
  /* map apply functions */
  const typesToFunction = {
    [stateTransitionTypes.DATA_CONTRACT_CREATE]: applyDataContractCreateTransition,
    [stateTransitionTypes.DATA_CONTRACT_UPDATE]: applyDataContractUpdateTransition,
    [stateTransitionTypes.DOCUMENTS_BATCH]: applyDocumentsBatchTransition,
    [stateTransitionTypes.IDENTITY_CREATE]: applyIdentityCreateTransition,
    [stateTransitionTypes.IDENTITY_TOP_UP]: applyIdentityTopUpTransition,
    [stateTransitionTypes.IDENTITY_UPDATE]: applyIdentityUpdateTransition,
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
