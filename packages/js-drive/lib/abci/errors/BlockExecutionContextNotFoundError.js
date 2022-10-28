const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const AbstractAbciError = require('./AbstractAbciError');

class BlockExecutionContextNotFoundError extends AbstractAbciError {
  /**
   *
   * @param {string} message
   * @param {Object} chainlock
   */
  constructor(round) {
    super(grpcErrorCodes.INTERNAL, 'BlockExecutionContext not found', { round });
  }
}

module.exports = BlockExecutionContextNotFoundError;
