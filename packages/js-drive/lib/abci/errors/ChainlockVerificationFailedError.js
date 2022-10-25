const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const AbstractAbciError = require('./AbstractAbciError');

class ChainlockVerificationFailedError extends AbstractAbciError {
  /**
   *
   * @param {string} message
   * @param {Object} chainlock
   */
  constructor(message, chainlock) {
    super(grpcErrorCodes.INTERNAL, `ChainLock verification failed: ${message}`, { chainlock });
  }
}

module.exports = ChainlockVerificationFailedError;
