const {
  tendermint: {
    abci: {
      ResponseVerifyVoteExtension,
    },
    types: {
      VoteExtensionType,
    },
  },
} = require('@dashevo/abci/types');

const verifyStatus = {
  UNKNOWN: 0, // Unknown status. Returning this from the application is always an error.
  ACCEPT: 1, // Status that signals that the application finds the vote extension valid.
  REJECT: 2, // Status that signals that the application finds the vote extension invalid.
};

/**
 * @param {BlockExecutionContext} proposalBlockExecutionContext
 * @return {verifyVoteExtensionHandler}
 */
function verifyVoteExtensionHandlerFactory(proposalBlockExecutionContext) {
  /**
   * @typedef verifyVoteExtensionHandler
   *
   * @param {abci.RequestVerifyVoteExtension} request
   *
   * @return {Promise<abci.ResponseVerifyVoteExtension>}
   */
  async function verifyVoteExtensionHandler({ voteExtensions }) {
    const consensusLogger = proposalBlockExecutionContext.getConsensusLogger()
      .child({
        abciMethod: 'verifyVoteExtension',
      });

    consensusLogger.debug('VerifyVote ABCI method requested');

    const unsignedWithdrawalTransactionsMap = proposalBlockExecutionContext
      .getWithdrawalTransactionsMap();

    const voteExtensionsToCheck = Object.keys(unsignedWithdrawalTransactionsMap || {})
      .sort()
      .map((txHashHex) => ({
        type: VoteExtensionType.THRESHOLD_RECOVER,
        extension: Buffer.from(txHashHex, 'hex'),
      }));

    const allVoteExtensionsPresent = voteExtensionsToCheck.reduce((result, nextExtension) => {
      const searchedVoteExtension = (voteExtensions || []).find((voteExtension) => (
        voteExtension.type === nextExtension.type
        && Buffer.compare(voteExtension.extension, nextExtension.extension)
      ));

      if (!searchedVoteExtension) {
        const extensionString = nextExtension.extension.toString('hex');

        const extensionTruncatedString = extensionString.substring(
          0,
          Math.min(30, extensionString.length),
        );

        consensusLogger.warn({
          type: nextExtension.type,
          extension: extensionString,
        }, `${nextExtension.type} vote extension ${extensionTruncatedString}... was not found in verify request`);
      }

      return result && (searchedVoteExtension !== undefined);
    }, true);

    const status = allVoteExtensionsPresent ? verifyStatus.ACCEPT : verifyStatus.REJECT;

    return new ResponseVerifyVoteExtension({
      status,
    });
  }

  return verifyVoteExtensionHandler;
}

module.exports = verifyVoteExtensionHandlerFactory;
