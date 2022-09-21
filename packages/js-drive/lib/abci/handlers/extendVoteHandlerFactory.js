const {
  tendermint: {
    abci: {
      ResponseExtendVote,
    },
  },
} = require('@dashevo/abci/types');

const VOTE_EXTENSION_TYPES = {
  DEFAULT: 0,
  TRESHOLD: 1,
};

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
    const withdrawalTransactions = blockExecutionContext.getWithdrawalTransactions();

    const voteExtentions = withdrawalTransactions.map((txBytes) => ({
      type: VOTE_EXTENSION_TYPES.TRESHOLD,
      extension: txBytes,
    }));

    return new ResponseExtendVote({
      vote_extensions: voteExtentions,
    });
  }

  return extendVoteHandler;
}

module.exports = extendVoteHandlerFactory;
