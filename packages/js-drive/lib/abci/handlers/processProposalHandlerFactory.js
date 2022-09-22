const {
  tendermint: {
    abci: {
      ResponseProcessProposal,
    },
  },
} = require('@dashevo/abci/types');

const proposalStatus = {
  UNKNOWN: 0, // Unknown status. Returning this from the application is always an error.
  ACCEPT: 1, // Status that signals that the application finds the proposal valid.
  REJECT: 2, // Status that signals that the application finds the proposal invalid.
};

/**
 *
 * @return {processProposalHandler}
 */
function processProposalHandlerFactory() {
  /**
   * @typedef processProposalHandler
   * @return {Promise<abci.ResponseProcessProposal>}
   */
  async function processProposalHandler() {
    return new ResponseProcessProposal({
      status: proposalStatus.ACCEPT,
    });
  }

  return processProposalHandler;
}

module.exports = processProposalHandlerFactory;
