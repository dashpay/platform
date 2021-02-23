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
  const lastSyncedBlockHash = this.getLastSyncedBlockHash();
  const bestBlockHeight = await this.getBestBlockHeightFromTransport();
  const lastSyncedBlockHeight = await this.getLastSyncedBlockHeight();
  const count = bestBlockHeight - lastSyncedBlockHeight || 1;
  const start = +new Date();

  try {
    const options = { count, network };
    // If there's no blocks synced, start from height 0, otherwise from the last block hash.
    if (lastSyncedBlockHash == null) {
      options.fromBlockHeight = lastSyncedBlockHeight;
    } else {
      options.fromBlockHash = lastSyncedBlockHash;
    }

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

    this.stream = null;
    this.emit('error', e, {
      type: 'plugin',
      pluginType: 'worker',
      pluginName: this.name,
    });
  }

  this.setLastSyncedBlockHeight(bestBlockHeight);

  logger.debug(`TransactionSyncStreamWorker - HistoricalSync - Synchronized ${count} in ${+new Date() - start}ms`);
};
