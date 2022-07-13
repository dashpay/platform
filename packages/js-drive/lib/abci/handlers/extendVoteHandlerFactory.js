const {
  tendermint: {
    abci: {
      ResponseExtendVote,
    },
  },
} = require('@dashevo/abci/types');

/**
 *
 * @return {extendVoteHandler}
 */
function extendVoteHandlerFactory() {
  /**
   * @typedef extendVoteHandler
   * @return {Promise<abci.ResponseExtendVote>}
   */
  async function extendVoteHandler() {
    return new ResponseExtendVote({});
  }

  return extendVoteHandler;
}

module.exports = extendVoteHandlerFactory;
