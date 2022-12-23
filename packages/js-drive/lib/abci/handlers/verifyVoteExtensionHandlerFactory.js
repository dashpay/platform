const {
  tendermint: {
    abci: {
      ResponseVerifyVoteExtension,
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
   * @return {Promise<abci.ResponseVerifyVoteExtension>}
   */
  async function verifyVoteExtensionHandler() {
    const consensusLogger = proposalBlockExecutionContext.getConsensusLogger()
      .child({
        abciMethod: 'verifyVoteExtension',
      });

    consensusLogger.debug('VerifyVote ABCI method requested');

    // TODO Verify withdrawal vote extensions and add logs

    return new ResponseVerifyVoteExtension({
      status: verifyStatus.ACCEPT,
    });
  }

  return verifyVoteExtensionHandler;
}

module.exports = verifyVoteExtensionHandlerFactory;
