const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const logger = require('../../../../logger');

const GRPC_RETRY_ERRORS = [
  GrpcErrorCodes.DEADLINE_EXCEEDED,
  GrpcErrorCodes.UNAVAILABLE,
  GrpcErrorCodes.INTERNAL,
  GrpcErrorCodes.CANCELLED,
  GrpcErrorCodes.UNKNOWN,
];

module.exports = async function startIncomingSync() {
  const { network } = this;
  const lastSyncedBlockHeight = await this.getLastSyncedBlockHeight();
  const count = 0;

  try {
    const options = { count, network };
    options.fromBlockHeight = lastSyncedBlockHeight > 0 ? lastSyncedBlockHeight : 1;

    await this.syncUpToTheGapLimit(options);
    // The method above resolves only in two cases: the limit is reached or the server is closed.
    // In both cases, the stream needs to be restarted, unless syncIncomingTransactions is
    // set to false, which is signalling the worker not to restart stream.
    if (this.syncIncomingTransactions) {
      logger.debug(`TransactionSyncStreamWorker - IncomingSync - Restarted from height: ${lastSyncedBlockHeight}`);

      await startIncomingSync.call(this);
    }
  } catch (e) {
    this.stream = null;

    if (GRPC_RETRY_ERRORS.includes(e.code)) {
      logger.debug(`TransactionSyncStreamWorker - IncomingSync - Restarted from height: ${lastSyncedBlockHeight}`);

      if (this.syncIncomingTransactions) {
        await startIncomingSync.call(this);
      }

      return;
    }

    this.emit('error', e, {
      type: 'plugin',
      pluginType: 'worker',
      pluginName: this.name,
    });
  }
};
