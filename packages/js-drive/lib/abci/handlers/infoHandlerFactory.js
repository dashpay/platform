const {
  tendermint: {
    abci: {
      ResponseInfo,
    },
  },
} = require('@dashevo/abci/types');

const { version: driveVersion } = require('../../../package');

/**
 * @param {ChainInfo} chainInfo
 * @param {Number} protocolVersion
 * @param {RootTree} rootTree
 * @return {infoHandler}
 */
function infoHandlerFactory(chainInfo, protocolVersion, rootTree) {
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
      lastBlockAppHash: rootTree.getRootHash(),
    });
  }

  return infoHandler;
}

module.exports = infoHandlerFactory;
