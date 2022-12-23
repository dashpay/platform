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
   * @return {Promise<abci.ResponseExtendVote>}
   */
  async function extendVoteHandler() {
    const consensusLogger = proposalBlockExecutionContext.getConsensusLogger()
      .child({
        abciMethod: 'extendVote',
      });

    consensusLogger.debug('ExtendVote ABCI method requested');

    const unsignedWithdrawalTransactionsMap = proposalBlockExecutionContext
      .getWithdrawalTransactionsMap();

    const voteExtensions = Object.keys(unsignedWithdrawalTransactionsMap)
      .sort()
      .map((txHashHex) => ({
        type: VoteExtensionType.THRESHOLD_RECOVER,
        extension: Buffer.from(txHashHex, 'hex'),
      }));

    const voteExtensionTypeName = {
      [VoteExtensionType.DEFAULT]: 'default',
      [VoteExtensionType.THRESHOLD_RECOVER]: 'threshold recovery',
    };

    voteExtensions.forEach(({ extension, type }) => {
      const extensionString = extension.toString('hex');

      const extensionTruncatedString = extensionString.substring(
        0,
        Math.min(30, extensionString.length),
      );

      consensusLogger.debug({
        type,
        extension: extensionString,
      }, `Vote extended to obtain ${voteExtensionTypeName} signature for ${extensionTruncatedString}... payload`);
    });

    return new ResponseExtendVote({
      voteExtensions,
    });
  }

  return extendVoteHandler;
}

module.exports = extendVoteHandlerFactory;
