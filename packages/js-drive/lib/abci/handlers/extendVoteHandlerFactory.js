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
 * @param {BlockExecutionContext} blockExecutionContext
 *
 * @return {extendVoteHandler}
 */
function extendVoteHandlerFactory(blockExecutionContext) {
  /**
   * @typedef extendVoteHandler
   * @return {Promise<abci.ResponseExtendVote>}
   */
  async function extendVoteHandler() {
    const unsignedWithdrawalTransactionsMap = blockExecutionContext.getWithdrawalTransactionsMap();

    const voteExtentions = Object.keys(unsignedWithdrawalTransactionsMap)
      .sort()
      .map((txHashHex) => ({
        type: VoteExtensionType.THRESHOLD_RECOVER,
        extension: Buffer.from(txHashHex, 'hex'),
      }));

    return new ResponseExtendVote({
      vote_extensions: voteExtentions,
    });
  }

  return extendVoteHandler;
}

module.exports = extendVoteHandlerFactory;
