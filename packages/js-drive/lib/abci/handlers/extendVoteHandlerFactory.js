const {
  tendermint: {
    abci: {
      ResponseExtendVote,
    },
    types: {
      VoteExtensionType,
    },
  },
} = require('@dashevo/abci/types');

/**
 * @param {BlockExecutionContext} latestBlockExecutionContext
 *
 * @return {extendVoteHandler}
 */
function extendVoteHandlerFactory(latestBlockExecutionContext) {
  /**
   * @typedef extendVoteHandler
   * @return {Promise<abci.ResponseExtendVote>}
   */
  async function extendVoteHandler() {
    const unsignedWithdrawalTransactionsMap = latestBlockExecutionContext
      .getWithdrawalTransactionsMap();

    const voteExtentions = Object.keys(unsignedWithdrawalTransactionsMap)
      .sort()
      .map((txHashHex) => ({
        type: VoteExtensionType.THRESHOLD_RECOVER,
        extension: Buffer.from(txHashHex, 'hex'),
      }));

    return new ResponseExtendVote({
      voteExtensions: voteExtentions,
    });
  }

  return extendVoteHandler;
}

module.exports = extendVoteHandlerFactory;
