const {
  abci: {
    ResponseInfo,
  },
} = require('abci/types');

const { version: driveVersion } = require('../../../package');

/**
 * @param {BlockchainState} blockchainState
 * @param {Number} protocolVersion
 * @return {infoHandler}
 */
function infoHandlerFactory(blockchainState, protocolVersion) {
  /**
   * Info ABCI handler
   *
   * @typedef infoHandler
   *
   * @return {Promise<ResponseInfo>}
   */
  async function infoHandler() {
    return new ResponseInfo({
      version: driveVersion,
      appVersion: protocolVersion,
      lastBlockHeight: blockchainState.getLastBlockHeight(),
      lastBlockAppHash: blockchainState.getLastBlockAppHash(),
    });
  }

  return infoHandler;
}

module.exports = infoHandlerFactory;
