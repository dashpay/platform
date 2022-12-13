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
 * @param {ProposalBlockExecutionContextCollection} proposalBlockExecutionContextCollection
 *
 * @return {verifyVoteExtensionHandler}
 */
function verifyVoteExtensionHandlerFactory(proposalBlockExecutionContextCollection) {
  /**
   * @typedef verifyVoteExtensionHandler
   * @return {Promise<abci.ResponseVerifyVoteExtension>}
   */
  async function verifyVoteExtensionHandler() {
    const proposalBlockExecutionContext = proposalBlockExecutionContextCollection.get(round);
    const unsignedWithdrawalTransactionsMap = proposalBlockExecutionContext
      .getWithdrawalTransactionsMap();

    return new ResponseVerifyVoteExtension({
      status: verifyStatus.ACCEPT,
    });
  }

  return verifyVoteExtensionHandler;
}

module.exports = verifyVoteExtensionHandlerFactory;
