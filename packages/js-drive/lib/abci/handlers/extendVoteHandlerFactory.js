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
 * @param {BlockExecutionContext} proposalBlockExecutionContext
 *
 * @return {extendVoteHandler}
 */
function extendVoteHandlerFactory(proposalBlockExecutionContext) {
  /**
   * @typedef extendVoteHandler
   * @param {Object} request
   * @param {number} request.round
   * @return {Promise<abci.ResponseExtendVote>}
   */
  async function extendVoteHandler({ round }) {
    const unsignedWithdrawalTransactionsMap = proposalBlockExecutionContext
      .getWithdrawalTransactionsMap();

    const voteExtensions = Object.keys(unsignedWithdrawalTransactionsMap)
      .sort()
      .map((txHashHex) => ({
        type: VoteExtensionType.THRESHOLD_RECOVER,
        extension: Buffer.from(txHashHex, 'hex'),
      }));

    return new ResponseExtendVote({
      voteExtensions,
    });
  }

  return extendVoteHandler;
}

module.exports = extendVoteHandlerFactory;
