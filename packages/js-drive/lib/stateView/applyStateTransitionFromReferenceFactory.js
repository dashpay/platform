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
   * @param {boolean} reverting
   * @returns {Promise<void>}
   */
  async function applyStateTransitionFromReference({ blockHash, stHeaderHash }, reverting) {
    const [{ result: block }, { result: header }] = await Promise.all([
      rpcClient.getBlock(blockHash),
      rpcClient.getRawTransaction(stHeaderHash),
    ]);
    await applyStateTransition(header, block, reverting);
  }

  return applyStateTransitionFromReference;
};
