const {
  tendermint: {
    abci: {
      ResponseInfo,
    },
  },
} = require('@dashevo/abci/types');

const { asValue } = require('awilix');

const { version: driveVersion } = require('../../../package');

/**
 * @param {ChainInfoRepository} chainInfoRepository
 * @param {Number} protocolVersion
 * @param {RootTree} rootTree
 * @param {updateSimplifiedMasternodeList} updateSimplifiedMasternodeList
 * @param {BaseLogger} logger
 * @param {AwilixContainer} container
 * @return {infoHandler}
 */
function infoHandlerFactory(
  chainInfoRepository,
  protocolVersion,
  rootTree,
  updateSimplifiedMasternodeList,
  logger,
  container,
) {
  /**
   * Info ABCI handler
   *
   * @typedef infoHandler
   *
   * @param {abci.RequestDeliverTx} request
   * @return {Promise<ResponseInfo>}
   */
  async function infoHandler(request) {
    let contextLogger = logger.child({
      abciMethod: 'info',
    });

    contextLogger.debug('Info ABCI method requested');
    contextLogger.trace({ request });

    const chainInfo = await chainInfoRepository.fetch();

    container.register({
      chainInfo: asValue(chainInfo),
    });

    contextLogger = contextLogger.child({
      height: chainInfo.getLastBlockHeight().toString(),
    });

    // Update SML store to latest saved core chain lock to make sure
    // that verfy chain lock handler has updated SML Store to verify signatures
    if (chainInfo.getLastBlockHeight().gt(0)) {
      const lastCoreChainLockedHeight = chainInfo.getLastCoreChainLockedHeight();

      await updateSimplifiedMasternodeList(lastCoreChainLockedHeight, {
        logger: contextLogger,
      });
    }

    const appHash = rootTree.getRootHash();

    contextLogger.info(
      {
        ...chainInfo.toJSON(),
        appHash: appHash.toString('hex').toUpperCase(),
      },
      `Start processing from block #${chainInfo.getLastBlockHeight()} with appHash ${appHash.toString('hex').toUpperCase() || 'nil'}`,
    );

    return new ResponseInfo({
      version: driveVersion,
      appVersion: protocolVersion,
      lastBlockHeight: chainInfo.getLastBlockHeight(),
      lastBlockAppHash: appHash,
    });
  }

  return infoHandler;
}

module.exports = infoHandlerFactory;
