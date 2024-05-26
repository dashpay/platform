const MasternodeListSync = require('../../../MasternodeListSync');

/**
 * @param {MasternodeListSync} masternodeListSync
 */
function subscribeToMasternodeListHandlerFactory(masternodeListSync) {
  /**
   * @param {grpc.ServerWriteableStream<BlockHeadersWithChainLocksRequest>} call
   * @return {Promise<void>}
   */
  async function subscribeToMasternodeListHandler(call) {
    /**
     * @param {SimplifiedMNListDiff} diff
     */
    const sendDiff = (diff) => {
      call.write(diff.toBuffer());
    };

    const shutdown = () => {
      call.end();

      masternodeListSync.removeListener(MasternodeListSync.EVENT_DIFF, sendDiff);
    };

    call.on('end', shutdown);
    call.on('cancelled', shutdown);

    masternodeListSync.on(MasternodeListSync.EVENT_DIFF, sendDiff);

    sendDiff(masternodeListSync.getFullList());
  }

  return subscribeToMasternodeListHandler;
}

module.exports = subscribeToMasternodeListHandlerFactory;
