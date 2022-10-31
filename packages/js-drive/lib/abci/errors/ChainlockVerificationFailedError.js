const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const AbstractAbciError = require('./AbstractAbciError');

class ChainlockVerificationFailedError extends AbstractAbciError {
  /**
   *
   * @param {string} message
   * @param {Object} data
   */
  constructor(message, data) {
    super(grpcErrorCodes.INTERNAL, `ChainLock verification failed: ${message}`, data);
  }
}

module.exports = ChainlockVerificationFailedError;
