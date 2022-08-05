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
 *
 * @return {verifyVoteExtensionHandler}
 */
function verifyVoteExtensionHandlerFactory() {
  /**
   * @typedef verifyVoteExtensionHandler
   * @return {Promise<abci.ResponseVerifyVoteExtension>}
   */
  async function verifyVoteExtensionHandler() {
    return new ResponseVerifyVoteExtension({
      status: verifyStatus.ACCEPT,
    });
  }

  return verifyVoteExtensionHandler;
}

module.exports = verifyVoteExtensionHandlerFactory;
