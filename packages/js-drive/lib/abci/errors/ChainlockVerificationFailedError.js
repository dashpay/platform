const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const AbstractAbciError = require('./AbstractAbciError');

class ChainlockVerificationFailedError extends AbstractAbciError {
  /**
   *
   * @param {string} message
   * @param {number} code
   */
  constructor(message, code) {
    super(grpcErrorCodes.INTERNAL, `Chainlock verification failed using verifyChainLock method: ${message}`, { code });

    this.code = code;
  }

  /**
   * @returns {number}
   */
  getCode() {
    return this.code;
  }
}

module.exports = ChainlockVerificationFailedError;
