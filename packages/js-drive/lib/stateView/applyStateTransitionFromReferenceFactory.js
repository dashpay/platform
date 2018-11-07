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
   * @returns {Promise<void>}
   */
  async function applyStateTransitionFromReference({ blockHash, stHeaderHash }) {
    const [{ result: block }, { result: header }] = await Promise.all([
      rpcClient.getBlock(blockHash),
      rpcClient.getRawTransaction(stHeaderHash),
    ]);
    await applyStateTransition(header, block);
  }

  return applyStateTransitionFromReference;
};
