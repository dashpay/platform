const {
  abci: {
    ResponseInfo,
  },
} = require('abci/types');

const packageJson = require('../../../package');

/**
 * @param {BlockchainState} blockchainState
 * @return {infoHandler}
 */
function infoHandlerFactory(blockchainState) {
  /**
   * Info ABCI handler
   *
   * @typedef infoHandler
   *
   * @return {Promise<ResponseInfo>}
   */
  async function infoHandler() {
    return new ResponseInfo({
      version: packageJson.version,
      lastBlockHeight: blockchainState.getLastBlockHeight(),
      lastBlockAppHash: blockchainState.getLastBlockAppHash(),
    });
  }

  return infoHandler;
}

module.exports = infoHandlerFactory;
