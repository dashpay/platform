const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const logger = require('../../../../logger');

const GRPC_RETRY_ERRORS = [
  GrpcErrorCodes.DEADLINE_EXCEEDED,
  GrpcErrorCodes.UNAVAILABLE,
  GrpcErrorCodes.INTERNAL,
  GrpcErrorCodes.CANCELLED,
  GrpcErrorCodes.UNKNOWN,
];

/**
 *
 * @param {string} network
 * @return {Promise<void>}
 */
module.exports = async function startHistoricalSync(network) {
  const chainStore = this.storage.getChainStore(this.network.toString());
  const bestBlockHeight = chainStore.state.blockHeight;

  const lastSyncedBlockHeight = await this.getLastSyncedBlockHeight();
  const fromBlockHeight = lastSyncedBlockHeight > 0 ? lastSyncedBlockHeight : 1;
  const count = bestBlockHeight - fromBlockHeight || 1;
  const start = +new Date();

  try {
    const options = { count, network };
    options.fromBlockHeight = lastSyncedBlockHeight > 0 ? lastSyncedBlockHeight : 1;

    logger.debug(`TransactionSyncStreamWorker - HistoricalSync - Started from ${options.fromBlockHash || options.fromBlockHeight}, count: ${count}`);
    const gapLimitIsReached = await this.syncUpToTheGapLimit(options);
    if (gapLimitIsReached) {
      await startHistoricalSync.call(this, network);
    }
  } catch (e) {
    if (GRPC_RETRY_ERRORS.includes(e.code)) {
      if (this.stream === null && e.code === GrpcErrorCodes.CANCELLED) {
        // NOOP on self canceled state (via stop worker)
        logger.debug('TransactionSyncStreamWorker - HistoricalSync - The Worker is stopped');
        return;
      }

      logger.debug('TransactionSyncStreamWorker - HistoricalSync - Restarting the stream');

      this.stream = null;
      await startHistoricalSync.call(this, network);

      return;
    }

    // TODO: finish this function only when all this.transactionsToVerify were processed
    this.stream = null;
    this.emit('error', e, {
      type: 'plugin',
      pluginType: 'worker',
      pluginName: this.name,
    });
  }

  // TODO: remove "true" and move control over that to ChainSyncMediator
  this.setLastSyncedBlockHeight(bestBlockHeight, true);

  logger.debug(`TransactionSyncStreamWorker - HistoricalSync - Synchronized ${count} in ${+new Date() - start}ms`);
};
