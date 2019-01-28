/**
 * Create applyStateTransitionFromReference
 * @param {applyStateTransition} applyStateTransition
 * @param {RpcClient} rpcClient
 * @returns {applyStateTransitionFromReference}
 */
module.exports = function applyStateTransitionFromReferenceFactory(
  applyStateTransition,
  rpcClient,
) {
  /**
   * @typedef applyStateTransitionFromReference
   * @param {Reference} reference
   * @param {boolean} [reverting]
   * @returns {Promise<void>}
   */
  async function applyStateTransitionFromReference({ blockHash, stHash }, reverting = false) {
    const [{ result: block }, { result: stateTransition }] = await Promise.all([
      rpcClient.getBlock(blockHash),
      rpcClient.getRawTransaction(stHash),
    ]);

    await applyStateTransition(stateTransition, block, reverting);
  }

  return applyStateTransitionFromReference;
};
