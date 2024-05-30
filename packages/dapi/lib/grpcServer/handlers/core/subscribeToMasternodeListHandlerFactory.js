const {
  v0: {
    MasternodeListResponse,
  },
} = require('@dashevo/dapi-grpc');

const MasternodeListSync = require('../../../MasternodeListSync');
const logger = require('../../../logger');

/**
 * @param {MasternodeListSync} masternodeListSync
 */
function subscribeToMasternodeListHandlerFactory(masternodeListSync) {
  /**
   * @param {grpc.ServerWriteableStream<BlockHeadersWithChainLocksRequest>} call
   * @return {Promise<void>}
   */
  async function subscribeToMasternodeListHandler(call) {
    const requestLogger = logger.child({
      endpoint: 'subscribeToMasternodeListHandler',
      request: Math.floor(Math.random() * 1000),
    });

    requestLogger.debug('Start stream');

    // We create a closure here to have an independent listener for each call,
    // so we can easily remove it when the call ends
    const sendDiff = (diffBuffer, blockHeight, blockHash, full) => {
      const response = new MasternodeListResponse();

      response.setMasternodeListDiff(diffBuffer);

      let message = 'Masternode list diff sent';
      if (full) {
        message = 'Full masternode list sent';
      }

      requestLogger.trace({
        blockHeight,
        blockHash,
      }, message);

      call.write(response);
    };

    const shutdown = () => {
      call.end();

      requestLogger.trace('Shutdown stream');

      masternodeListSync.removeListener(MasternodeListSync.EVENT_DIFF, sendDiff);
    };

    call.on('end', shutdown);
    call.on('cancelled', shutdown);

    masternodeListSync.on(MasternodeListSync.EVENT_DIFF, sendDiff);

    // Send full masternode list on subscribe
    sendDiff(
      masternodeListSync.getFullDiffBuffer(),
      masternodeListSync.getBlockHeight(),
      masternodeListSync.getBlockHash(),
      true,
    );
  }

  return subscribeToMasternodeListHandler;
}

module.exports = subscribeToMasternodeListHandlerFactory;
