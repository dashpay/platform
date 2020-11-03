const {
  abci: {
    ResponseInfo,
  },
} = require('abci/types');

const { version: driveVersion } = require('../../../package');

/**
 * @param {ChainInfo} chainInfo
 * @param {Number} protocolVersion
 * @return {infoHandler}
 */
function infoHandlerFactory(chainInfo, protocolVersion) {
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
      lastBlockHeight: chainInfo.getLastBlockHeight(),
      lastBlockAppHash: chainInfo.getLastBlockAppHash(),
    });
  }

  return infoHandler;
}

module.exports = infoHandlerFactory;
